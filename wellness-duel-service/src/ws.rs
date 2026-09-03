use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;

use crate::rooms::{build_room_state, fetch_room};
use crate::state::AppState;

/// The socket never reads game actions from the client — all writes go
/// through the REST checkin endpoint, which validates and recomputes
/// everything server-side. This connection's only job is to push a fresh
/// RoomState the moment anyone in the room checks in, plus answer pings so
/// the connection stays alive.
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

    actix_web::rt::spawn(async move {
        if let Some(s) = initial_state {
            if let Ok(json) = serde_json::to_string(&s) {
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
