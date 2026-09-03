use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------- DB row shapes ----------

#[derive(Debug, sqlx::FromRow)]
pub struct RoomRow {
    pub id: Uuid,
    pub code: String,
    pub max_days: i32,
    pub win_target: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PlayerRow {
    pub id: Uuid,
    pub room_id: Uuid,
    pub device_token: String,
    pub name: String,
    pub streak: i32,
    pub total_points: i32,
    pub last_activity_key: Option<String>,
    pub repeat_count: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CheckinRow {
    pub day: i32,
    pub activity_key: Option<String>,
    pub points: i32,
    pub skipped: bool,
    pub photo_path: Option<String>,
}

// ---------- Public JSON shapes ----------
// The same `RoomState` shape is returned by create-room, join, checkin, the
// plain GET, and every WebSocket push — one canonical shape, everywhere.

#[derive(Debug, Serialize)]
pub struct RoomMeta {
    pub code: String,
    pub max_days: i32,
    pub win_target: i32,
    pub current_day: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CheckinView {
    pub day: i32,
    pub activity_key: Option<String>,
    pub points: i32,
    pub skipped: bool,
    pub photo_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlayerView {
    pub id: Uuid,
    pub name: String,
    pub streak: i32,
    pub total_points: i32,
    pub history: Vec<CheckinView>,
}

#[derive(Debug, Serialize)]
pub struct RoomState {
    pub room: RoomMeta,
    pub players: Vec<PlayerView>,
}

// ---------- Request DTOs ----------

#[derive(Debug, Deserialize)]
pub struct CreateRoomReq {
    pub max_days: Option<i32>,
    pub win_target: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct JoinReq {
    pub device_token: String,
    pub name: String,
}

/// Join is the one endpoint that returns something beyond the plain
/// RoomState: the client needs to know which player_id is "me" so it can
/// show action buttons only on its own card once a room has more than two
/// people in it. Every other endpoint (create, checkin, GET state, and every
/// WebSocket push) returns RoomState directly.
#[derive(Debug, Serialize)]
pub struct JoinResp {
    pub player_id: Uuid,
    pub state: RoomState,
}
