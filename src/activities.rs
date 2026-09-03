/// The server is the single source of truth for points. The client mirrors
/// this list for its activity picker, but a client can never award itself
/// points directly — every checkin is recomputed here from scratch.
#[allow(dead_code)]
pub struct Activity {
    pub key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub points: i32,
}

pub const ACTIVITIES: &[Activity] = &[
    Activity { key: "squats",  name: "10 Squats",             emoji: "💪", points: 3 },
    Activity { key: "walk",    name: "5-Minute Walk",          emoji: "🚶", points: 2 },
    Activity { key: "stretch", name: "2-Minute Stretch",       emoji: "🧘", points: 2 },
    Activity { key: "water",   name: "Drink a Glass of Water", emoji: "💧", points: 1 },
    Activity { key: "dance",   name: "Dance to One Song",      emoji: "💃", points: 3 },
    Activity { key: "breathe", name: "10 Deep Breaths",        emoji: "❤️", points: 1 },
    Activity { key: "rolls",   name: "Shoulder & Neck Rolls",  emoji: "⚡", points: 1 },
];

pub fn find_activity(key: &str) -> Option<&'static Activity> {
    ACTIVITIES.iter().find(|a| a.key == key)
}

/// Repeating yesterday's activity earns reduced points, encouraging variety.
/// repeat_count is how many consecutive days (including today) this same
/// activity has now been chosen. 0 = first time / not a repeat.
pub fn repeat_multiplier(repeat_count: i32) -> f64 {
    match repeat_count {
        0 => 1.0,
        1 => 0.7,
        2 => 0.4,
        _ => 0.2,
    }
}

/// Never award zero points for a genuinely completed activity, even after
/// heavy repeat decay — a completed task should always feel like it counted.
pub fn compute_points(base: i32, repeat_count: i32) -> i32 {
    let raw = (base as f64) * repeat_multiplier(repeat_count);
    raw.round().max(1.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_decay_matches_design() {
        assert_eq!(compute_points(3, 0), 3);
        assert_eq!(compute_points(3, 1), 2); // 3 * 0.7 = 2.1 -> 2
        assert_eq!(compute_points(3, 2), 1); // 3 * 0.4 = 1.2 -> 1
        assert_eq!(compute_points(1, 5), 1); // floor of 1 point, never 0
    }
}
