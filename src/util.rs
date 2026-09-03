use chrono::{DateTime, Utc};
use rand::Rng;

// Excludes visually ambiguous characters (0/O, 1/I/L) so a code is easy to
// read aloud or copy off a phone screen.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

pub fn generate_room_code() -> String {
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CODE_ALPHABET.len());
            CODE_ALPHABET[idx] as char
        })
        .collect()
}

/// Days are tied to real elapsed calendar time from room creation, not a
/// client-controlled "next day" button — day 1 is creation day, day 2 is the
/// next calendar day, and so on. This is what makes the server authoritative
/// about pacing: nobody can rapid-fire through a week in ten seconds.
pub fn compute_current_day(created_at: DateTime<Utc>) -> i32 {
    let elapsed_days = (Utc::now().date_naive() - created_at.date_naive()).num_days();
    (elapsed_days + 1) as i32
}

/// Personal recovery code — 8 lowercase alphanumeric characters, visually
/// distinct from the 6-char uppercase room codes so players never confuse them.
const RECOVERY_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

pub fn generate_recovery_code() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..RECOVERY_ALPHABET.len());
            RECOVERY_ALPHABET[idx] as char
        })
        .collect()
}
