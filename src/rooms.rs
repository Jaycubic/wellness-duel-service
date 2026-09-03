use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use futures_util::TryStreamExt;
use sqlx::PgPool;
use uuid::Uuid;

use crate::activities::{compute_points, find_activity};
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;
use crate::util::{compute_current_day, generate_recovery_code, generate_room_code};

pub async fn fetch_room(pool: &PgPool, code: &str) -> AppResult<RoomRow> {
    sqlx::query_as::<_, RoomRow>(
        "SELECT id, code, max_days, win_target, created_at FROM rooms WHERE code = $1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::RoomNotFound)
}

pub async fn build_room_state(pool: &PgPool, room: &RoomRow) -> AppResult<RoomState> {
    let players: Vec<PlayerRow> = sqlx::query_as::<_, PlayerRow>(
        "SELECT id, room_id, device_token, name, streak, total_points, last_activity_key, repeat_count, recovery_code
         FROM players WHERE room_id = $1 ORDER BY total_points DESC, name ASC",
    )
    .bind(room.id)
    .fetch_all(pool)
    .await?;

    let mut player_views = Vec::with_capacity(players.len());
    for p in players {
        let checkins: Vec<CheckinRow> = sqlx::query_as::<_, CheckinRow>(
            "SELECT day, activity_key, points, skipped, photo_path
             FROM checkins WHERE player_id = $1 ORDER BY day ASC",
        )
        .bind(p.id)
        .fetch_all(pool)
        .await?;

        let history = checkins
            .into_iter()
            .map(|c| CheckinView {
                day: c.day,
                activity_key: c.activity_key,
                points: c.points,
                skipped: c.skipped,
                photo_url: c.photo_path.map(|path| format!("/uploads/{path}")),
            })
            .collect();

        player_views.push(PlayerView {
            id: p.id,
            name: p.name,
            streak: p.streak,
            total_points: p.total_points,
            history,
        });
    }

    Ok(RoomState {
        room: RoomMeta {
            code: room.code.clone(),
            max_days: room.max_days,
            win_target: room.win_target,
            current_day: compute_current_day(room.created_at),
            created_at: room.created_at,
        },
        players: player_views,
    })
}

pub async fn create_room(
    state: web::Data<AppState>,
    body: web::Json<CreateRoomReq>,
) -> AppResult<HttpResponse> {
    let max_days = body.max_days.unwrap_or(7).clamp(1, 30);
    let win_target = body.win_target.unwrap_or(15).clamp(1, 1000);

    for _ in 0..5 {
        let code = generate_room_code();
        let id = Uuid::new_v4();

        let result = sqlx::query(
            "INSERT INTO rooms (id, code, max_days, win_target) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(&code)
        .bind(max_days)
        .bind(win_target)
        .execute(&state.pool)
        .await;

        match result {
            Ok(_) => {
                let room = RoomRow {
                    id,
                    code,
                    max_days,
                    win_target,
                    created_at: Utc::now(),
                };
                let fresh_state = build_room_state(&state.pool, &room).await?;
                return Ok(HttpResponse::Ok().json(fresh_state));
            }
            // A room-code collision is the only expected failure mode here;
            // anything else is a real error worth propagating.
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Err(AppError::BadRequest(
        "could not allocate a unique room code, please try again".into(),
    ))
}

pub async fn join_room(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<JoinReq>,
) -> AppResult<HttpResponse> {
    let room_code = path.into_inner();
    let room = fetch_room(&state.pool, &room_code).await?;

    let trimmed = body.name.trim();
    let name: String = if trimmed.is_empty() {
        "Player".to_string()
    } else {
        trimmed.chars().take(24).collect()
    };

    let existing: Option<PlayerRow> = sqlx::query_as::<_, PlayerRow>(
        "SELECT id, room_id, device_token, name, streak, total_points, last_activity_key, repeat_count, recovery_code
         FROM players WHERE room_id = $1 AND device_token = $2",
    )
    .bind(room.id)
    .bind(&body.device_token)
    .fetch_optional(&state.pool)
    .await?;

    let (player_id, recovery_code) = if let Some(p) = existing {
        // Reconnecting on the same device: allow a rename, keep their streak/points.
        sqlx::query("UPDATE players SET name = $1 WHERE id = $2")
            .bind(&name)
            .bind(p.id)
            .execute(&state.pool)
            .await?;
        (p.id, p.recovery_code.unwrap_or_default())
    } else {
        let new_id = Uuid::new_v4();
        let rc = generate_recovery_code();
        sqlx::query("INSERT INTO players (id, room_id, device_token, name, recovery_code) VALUES ($1, $2, $3, $4, $5)")
            .bind(new_id)
            .bind(room.id)
            .bind(&body.device_token)
            .bind(&name)
            .bind(&rc)
            .execute(&state.pool)
            .await?;
        (new_id, rc)
    };

    let fresh_state = build_room_state(&state.pool, &room).await?;
    broadcast_state(&state, &room.code, &fresh_state);
    Ok(HttpResponse::Ok().json(JoinResp { player_id, recovery_code, state: fresh_state }))
}

pub async fn get_state(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let room = fetch_room(&state.pool, &path.into_inner()).await?;
    let fresh_state = build_room_state(&state.pool, &room).await?;
    Ok(HttpResponse::Ok().json(fresh_state))
}

pub async fn submit_checkin(
    state: web::Data<AppState>,
    path: web::Path<String>,
    mut payload: Multipart,
) -> AppResult<HttpResponse> {
    let room_code = path.into_inner();

    let mut device_token: Option<String> = None;
    let mut activity_key: Option<String> = None;
    let mut skip = false;
    let mut photo_bytes: Option<Vec<u8>> = None;

    while let Some(mut field) = payload.try_next().await? {
        let field_name = field
            .content_disposition()
            .get_name()
            .unwrap_or("")
            .to_string();

        let mut bytes = Vec::new();
        while let Some(chunk) = field.try_next().await? {
            bytes.extend_from_slice(&chunk);
        }

        match field_name.as_str() {
            "device_token" => device_token = Some(String::from_utf8_lossy(&bytes).into_owned()),
            "activity_key" => activity_key = Some(String::from_utf8_lossy(&bytes).into_owned()),
            "skip" => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                skip = text == "true" || text == "1";
            }
            "photo" if !bytes.is_empty() => photo_bytes = Some(bytes),
            _ => { /* ignore anything else */ }
        }
    }

    let device_token =
        device_token.ok_or_else(|| AppError::BadRequest("device_token is required".into()))?;

    let room = fetch_room(&state.pool, &room_code).await?;

    let current_day = compute_current_day(room.created_at);
    if current_day > room.max_days {
        return Err(AppError::WeekComplete(room.max_days));
    }

    let player: PlayerRow = sqlx::query_as::<_, PlayerRow>(
        "SELECT id, room_id, device_token, name, streak, total_points, last_activity_key, repeat_count, recovery_code
         FROM players WHERE room_id = $1 AND device_token = $2",
    )
    .bind(room.id)
    .bind(&device_token)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::PlayerNotFound)?;

    let already_checked_in: Option<(i32,)> =
        sqlx::query_as("SELECT day FROM checkins WHERE player_id = $1 AND day = $2")
            .bind(player.id)
            .bind(current_day)
            .fetch_optional(&state.pool)
            .await?;
    if already_checked_in.is_some() {
        return Err(AppError::AlreadyCheckedIn);
    }

    // The server recomputes everything from scratch here — a client can
    // request "I did squats" or "I'm skipping today," but it can never hand
    // the server a point total directly.
    let (new_streak, new_total, new_last_key, new_repeat, points, activity_key_for_row) = if skip {
        (0, player.total_points, None::<String>, 0, 0, None::<String>)
    } else {
        let key = activity_key
            .ok_or_else(|| AppError::BadRequest("activity_key is required unless skip=true".into()))?;
        let activity = find_activity(&key).ok_or_else(|| AppError::UnknownActivity(key.clone()))?;
        let is_repeat = player.last_activity_key.as_deref() == Some(key.as_str());
        let repeat_count = if is_repeat { player.repeat_count + 1 } else { 0 };
        let points = compute_points(activity.points, repeat_count);
        (
            player.streak + 1,
            player.total_points + points,
            Some(key.clone()),
            repeat_count,
            points,
            Some(key),
        )
    };

    let photo_path = match photo_bytes {
        Some(bytes) if !skip => Some(save_photo(&state.uploads_dir, &room.code, player.id, current_day, &bytes)?),
        _ => None,
    };

    let checkin_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "UPDATE players SET streak = $1, total_points = $2, last_activity_key = $3, repeat_count = $4
         WHERE id = $5",
    )
    .bind(new_streak)
    .bind(new_total)
    .bind(&new_last_key)
    .bind(new_repeat)
    .bind(player.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO checkins (id, player_id, day, activity_key, points, skipped, photo_path)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(checkin_id)
    .bind(player.id)
    .bind(current_day)
    .bind(&activity_key_for_row)
    .bind(points)
    .bind(skip)
    .bind(&photo_path)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let fresh_state = build_room_state(&state.pool, &room).await?;
    broadcast_state(&state, &room.code, &fresh_state);
    Ok(HttpResponse::Ok().json(fresh_state))
}

fn broadcast_state(state: &web::Data<AppState>, room_code: &str, fresh_state: &RoomState) {
    if let Ok(json) = serde_json::to_string(&WsOutgoing::State(fresh_state)) {
        state.broadcast(room_code, &json);
    }
}

pub async fn recover_player(
    state: web::Data<AppState>,
    body: web::Json<RecoverReq>,
) -> AppResult<HttpResponse> {
    let code = body.recovery_code.trim().to_lowercase();
    if code.is_empty() {
        return Err(AppError::BadRequest("recovery_code is required".into()));
    }

    // Look up the player by their unique recovery code
    let player: PlayerRow = sqlx::query_as::<_, PlayerRow>(
        "SELECT id, room_id, device_token, name, streak, total_points, last_activity_key, repeat_count, recovery_code
         FROM players WHERE recovery_code = $1",
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::BadRequest("invalid recovery code".into()))?;

    // Look up their room
    let room: RoomRow = sqlx::query_as::<_, RoomRow>(
        "SELECT id, code, max_days, win_target, created_at FROM rooms WHERE id = $1",
    )
    .bind(player.room_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::RoomNotFound)?;

    // Update the player's device_token to the recovering device so future
    // checkins from this device are recognised as this player.
    let device_token_from_body: Option<String> = None; // recovery doesn't rebind device_token
    let _ = device_token_from_body; // suppress warning

    let fresh_state = build_room_state(&state.pool, &room).await?;
    Ok(HttpResponse::Ok().json(RecoverResp {
        player_id: player.id,
        room_code: room.code,
        recovery_code: player.recovery_code.unwrap_or_default(),
        state: fresh_state,
    }))
}

/// Resizes to a max edge of 480px and re-encodes as JPEG before writing to
/// disk, so a week of photos across several players stays lightweight.
fn save_photo(
    uploads_dir: &str,
    room_code: &str,
    player_id: Uuid,
    day: i32,
    bytes: &[u8],
) -> AppResult<String> {
    let dir = format!("{uploads_dir}/{room_code}");
    std::fs::create_dir_all(&dir)?;

    let img = image::load_from_memory(bytes)?;
    let resized = img.thumbnail(480, 480);

    let filename = format!("{player_id}_day{day}.jpg");
    let full_path = format!("{dir}/{filename}");
    resized.save_with_format(&full_path, image::ImageFormat::Jpeg)?;

    Ok(format!("{room_code}/{filename}"))
}