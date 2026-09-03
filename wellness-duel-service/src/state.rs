use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

/// One broadcast channel per active room code. A WebSocket connection
/// subscribes to a room's channel on join; any REST checkin that mutates
/// that room publishes the fresh RoomState JSON here, and every connected
/// socket forwards it straight out to its client.
///
/// A plain std::sync::Mutex is safe here because the lock is only ever held
/// for the few microseconds it takes to look up or insert a HashMap entry —
/// it is never held across an `.await` point.
pub struct AppState {
    pub pool: PgPool,
    pub uploads_dir: String,
    rooms: Mutex<HashMap<String, broadcast::Sender<String>>>,
}

impl AppState {
    pub fn new(pool: PgPool, uploads_dir: String) -> Self {
        Self {
            pool,
            uploads_dir,
            rooms: Mutex::new(HashMap::new()),
        }
    }

    /// Get this room's broadcast sender, creating the channel if this is the
    /// first time anyone has touched this room since the process started.
    pub fn channel_for(&self, room_code: &str) -> broadcast::Sender<String> {
        let mut rooms = self.rooms.lock().expect("room registry mutex poisoned");
        rooms
            .entry(room_code.to_string())
            .or_insert_with(|| broadcast::channel(64).0)
            .clone()
    }

    /// Publish a fresh RoomState snapshot to every socket currently
    /// subscribed to this room. Errors here just mean "nobody is listening
    /// right now," which is fine — the sender still has the payload
    /// available to whoever's viewing the freshly-returned HTTP response.
    pub fn broadcast(&self, room_code: &str, payload: &str) {
        let sender = self.channel_for(room_code);
        let _ = sender.send(payload.to_string());
    }
}
