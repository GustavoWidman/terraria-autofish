use std::time::{Duration, Instant};

/// events that flow through the central queue.
///
/// *ephemeral* events (fish detections) carry an [`Instant`] and are silently
/// discarded when they are older than [`FISH_EVENT_TTL`].
/// *persistent* events (potion drinks, casts, hotkey commands) are never
/// discarded.
#[derive(Debug)]
pub enum FisherEvent {
    /// a fish was detected by the memory scanner. ephemeral, ignored/dropped if timestamp is older than [`FISH_EVENT_TTL`].
    FishDetected { fish_id: i32, timestamp: Instant },
    /// a potion timer expired – time to re-drink.
    DrinkPotion(super::potions::Potion),
    /// (re)cast the fishing rod. ephemeral, ignored/dropped if timestamp is older than [`FISH_EVENT_TTL`].
    Cast { timestamp: Instant },
    /// hotkey: toggle the macro on/off (hard stop).
    Toggle,
    /// hotkey: pause/resume (soft stop – timers keep ticking).
    Pause,
}

/// fish events older than this are silently dropped.
pub const FISH_EVENT_TTL: Duration = Duration::from_secs(1);
