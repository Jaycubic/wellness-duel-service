use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;

use chrono::Utc;
use uuid::Uuid;

use crate::models::{ChatMessageView, PlayerRow, WsIncoming, WsOutgoing};
use crate::rooms::{build_room_state, fetch_room};
use crate::state::AppState;

/// The socket pushes game state and chat messages to the client.
/// It also reads incoming chat frames from the client and broadcasts them
/// to the room after persisting them to the database.
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let room_code = path.into_inner();

    let room = match fetch_room(&state.pool, &room_code).await {
        Ok(r) => r,
        Err(_) => return Ok(HttpResponse::NotFound().body("unknown room code")),
    };

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let initial_state = build_room_state(&state.pool, &room).await.ok();
    let mut rx = state.channel_for(&room_code).subscribe();
    let pool = state.pool.clone();
    let room_id = room.id;
    let room_code_clone = room_code.clone();

    actix_web::rt::spawn(async move {
        if let Some(s) = initial_state {
            if let Ok(json) = serde_json::to_string(&WsOutgoing::State(&s)) {
                let _ = session.text(json).await;
            }
        }

        loop {
            tokio::select! {
                update = rx.recv() => {
                    match update {
                        Ok(json) => {
                            if session.text(json).await.is_err() {
                                break;
                            }
                        }
                        // A slow client fell behind the broadcast buffer; it
                        // will simply have a stale view until its next
                        // checkin or a manual refresh — not fatal.
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                                match incoming {
                                    WsIncoming::Chat { device_token, message } => {
                                        let msg = message.trim();
                                        if !msg.is_empty() && msg.len() <= 500 {
                                            // Look up player
                                            if let Ok(Some(player)) = sqlx::query_as::<_, PlayerRow>(
                                                "SELECT id, room_id, device_token, name, streak, total_points, last_activity_key, repeat_count, recovery_code
                                                 FROM players WHERE room_id = $1 AND device_token = $2",
                                            )
                                            .bind(room_id)
                                            .bind(&device_token)
                                            .fetch_optional(&pool)
                                            .await
                                            {
                                                let msg_id = Uuid::new_v4();
                                                let now = Utc::now();
                                                let _ = sqlx::query(
                                                    "INSERT INTO messages (id, room_id, player_id, sender_name, body, created_at)
                                                     VALUES ($1, $2, $3, $4, $5, $6)"
                                                )
                                                .bind(msg_id)
                                                .bind(room_id)
                                                .bind(player.id)
                                                .bind(&player.name)
                                                .bind(msg)
                                                .bind(now)
                                                .execute(&pool)
                                                .await;

                                                let view = ChatMessageView {
                                                    id: msg_id,
                                                    player_id: player.id,
                                                    sender_name: player.name.clone(),
                                                    body: msg.to_string(),
                                                    created_at: now,
                                                };

                                                if let Ok(json) = serde_json::to_string(&WsOutgoing::Chat(&view)) {
                                                    state.broadcast(&room_code_clone, &json);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Err(_)) => break,
                        _ => {}
                    }
                }
            }
        }

        let _ = session.close(None).await;
    });

    Ok(response)
}
