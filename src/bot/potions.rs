use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use log::debug;
use rdev::Key;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::event::FisherEvent;
use crate::utils::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Potion {
    Sonar,
    Fishing,
    Crate,
    Food,
}

impl std::fmt::Display for Potion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Potion::Sonar => write!(f, "Sonar"),
            Potion::Fishing => write!(f, "Fishing"),
            Potion::Crate => write!(f, "Crate"),
            Potion::Food => write!(f, "Food"),
        }
    }
}

/// per-potion state tracked by the scheduler.
struct PotionState {
    slot: Key,
    duration: Duration,
    /// `None` = never drunk / expired.  `Some(instant)` = last drink time.
    last_drunk: Option<Instant>,
}

/// schedules re-drink events for each enabled potion.
///
/// uses a [`CancellationToken`] so that **all** spawned timer tasks can be
/// killed at once when the macro is toggled off.  On each fresh activation
/// [`reset_and_schedule`] creates a new token and treats every potion as
/// never-drunk.
pub struct PotionScheduler {
    potions: HashMap<Potion, PotionState>,
    tx: mpsc::UnboundedSender<FisherEvent>,
    /// cancelled on toggle-off to kill every outstanding timer task.
    cancel: CancellationToken,
}

impl PotionScheduler {
    pub fn new(config: &Config, tx: mpsc::UnboundedSender<FisherEvent>) -> Self {
        let mut potions = HashMap::new();
        let pc = &config.fisher.potions;

        if let Some(slot) = pc.sonar_potion {
            potions.insert(
                Potion::Sonar,
                PotionState {
                    slot,
                    duration: Duration::from_secs(pc.sonar_potion_duration_secs),
                    last_drunk: None,
                },
            );
        }
        if let Some(slot) = pc.fishing_potion {
            potions.insert(
                Potion::Fishing,
                PotionState {
                    slot,
                    duration: Duration::from_secs(pc.fishing_potion_duration_secs),
                    last_drunk: None,
                },
            );
        }
        if let Some(slot) = pc.crate_potion {
            potions.insert(
                Potion::Crate,
                PotionState {
                    slot,
                    duration: Duration::from_secs(pc.crate_potion_duration_secs),
                    last_drunk: None,
                },
            );
        }

        if let Some(slot) = pc.food {
            potions.insert(
                Potion::Food,
                PotionState {
                    slot,
                    duration: Duration::from_secs(pc.food_duration_secs),
                    last_drunk: None,
                },
            );
        }

        Self {
            potions,
            tx,
            cancel: CancellationToken::new(),
        }
    }

    /// cancel **all** outstanding timer tasks and reset every potion to the
    /// "never drunk" state, as if the program had just started.
    ///
    /// must be called when the macro is toggled **off**.
    pub fn cancel_all(&mut self) {
        self.cancel.cancel();
        self.cancel = CancellationToken::new();

        for state in self.potions.values_mut() {
            state.last_drunk = None;
        }
        debug!("all potion timers cancelled and state reset");
    }

    /// check every enabled potion; for any that are expired (or never drunk),
    /// immediately enqueue a drink event. for those still active, spawn a
    /// delayed task (cancellable via the current token).
    pub fn schedule_all(&self) {
        let now = Instant::now();
        for (&potion, state) in &self.potions {
            let expired = state
                .last_drunk
                .map(|t| now.duration_since(t) >= state.duration)
                .unwrap_or(true);

            if expired {
                debug!("{} potion expired or never drunk, queueing drink", potion);
                let _ = self.tx.send(FisherEvent::DrinkPotion(potion));
            } else {
                let remaining = state.duration - now.duration_since(state.last_drunk.unwrap());
                let tx = self.tx.clone();
                let token = self.cancel.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        _ = token.cancelled() => {
                            debug!("{} potion timer cancelled", potion);
                        }
                        _ = tokio::time::sleep(remaining) => {
                            debug!("{} potion timer expired, queueing re-drink", potion);
                            let _ = tx.send(FisherEvent::DrinkPotion(potion));
                        }
                    }
                });
            }
        }
    }

    /// record that a potion was just drunk and schedule the next re-drink
    /// (cancellable via the current token).
    pub fn mark_drunk(&mut self, potion: Potion) {
        if let Some(state) = self.potions.get_mut(&potion) {
            let now = Instant::now();
            state.last_drunk = Some(now);

            let dur = state.duration;
            let tx = self.tx.clone();
            let token = self.cancel.clone();
            tokio::spawn(async move {
                tokio::select! {
                    _ = token.cancelled() => {
                        debug!("{} potion timer cancelled", potion);
                    }
                    _ = tokio::time::sleep(dur) => {
                        debug!("{} potion timer expired, queueing re-drink", potion);
                        let _ = tx.send(FisherEvent::DrinkPotion(potion));
                    }
                }
            });
        }
    }

    /// return the hotbar slot key for a given potion (if configured).
    pub fn slot_for(&self, potion: Potion) -> Option<Key> {
        self.potions.get(&potion).map(|s| s.slot)
    }

    /// whether any potions are configured at all.
    pub fn is_empty(&self) -> bool {
        self.potions.is_empty()
    }

    /// iterate over configured potions and their durations (for the welcome log).
    pub fn iter(&self) -> impl Iterator<Item = (Potion, Duration, Key)> + '_ {
        self.potions.iter().map(|(&p, s)| (p, s.duration, s.slot))
    }
}
