use actix_web::{web, HttpResponse};

use crate::error::{AppError, AppResult};
use crate::models::{ChatMessageRow, ChatMessageView};
use crate::rooms::fetch_room;
use crate::state::AppState;

pub async fn list_messages(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let room_code = path.into_inner();
    let room = fetch_room(&state.pool, &room_code).await?;

    let rows: Vec<ChatMessageRow> = sqlx::query_as::<_, ChatMessageRow>(
        "SELECT id, room_id, player_id, sender_name, body, created_at 
         FROM messages WHERE room_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(room.id)
    .fetch_all(&state.pool)
    .await?;

    // Reverse to send them chronologically
    let views: Vec<ChatMessageView> = rows
        .into_iter()
        .rev()
        .map(|r| ChatMessageView {
            id: r.id,
            player_id: r.player_id,
            sender_name: r.sender_name,
            body: r.body,
            created_at: r.created_at,
        })
        .collect();

    Ok(HttpResponse::Ok().json(views))
}
