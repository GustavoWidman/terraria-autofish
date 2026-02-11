use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use colored::Colorize;
use eyre::{Context, Result};
use log::{debug, info, warn};
use rdev::{EventType, Key};
use tokio::sync::mpsc;

use super::{
    event::{FISH_EVENT_TTL, FisherEvent},
    potions::PotionScheduler,
    scanner::MemoryScanner,
};
use crate::{
    bot::scanner::ScannerCommand,
    constants,
    utils::{
        config::{Config, HotkeyConfig},
        keyboard,
    },
};

/// spawn a background **OS thread** running [`rdev::listen`] that watches for
/// the configured toggle / pause hotkeys and forwards them as
/// [`FisherEvent`]s.
fn spawn_hotkey_listener(tx: mpsc::UnboundedSender<FisherEvent>, hotkeys: &HotkeyConfig) {
    let toggle_key = hotkeys.toggle;
    let pause_key = hotkeys.pause;

    std::thread::Builder::new()
        .name("hotkey-listener".into())
        .spawn(move || {
            if let Err(e) = rdev::listen(move |event| {
                if let EventType::KeyPress(key) = event.event_type {
                    if key == toggle_key {
                        let _ = tx.send(FisherEvent::Toggle);
                    } else if key == pause_key {
                        let _ = tx.send(FisherEvent::Pause);
                    }
                }
            }) {
                warn!("rdev listen error: {:?}", e);
            }
        })
        .expect("failed to spawn hotkey listener thread");
}

fn print_welcome(config: &Config, scheduler: &PotionScheduler) {
    info!("{}", "══════════════════════════════════════".bright_blue());
    info!("{}", "       terraria fishing macro".bright_blue().bold());
    info!("{}", "══════════════════════════════════════".bright_blue());

    // hotkeys
    info!(
        "  toggle: {:?}  |  pause: {:?}",
        config.fisher.hotkeys.toggle, config.fisher.hotkeys.pause
    );

    // potions
    let potions: Vec<_> = scheduler.iter().collect();
    if potions.is_empty() {
        info!("  potions: {}", "none configured".dimmed());
    } else {
        for (potion, dur, slot) in &potions {
            let mins = dur.as_secs() / 60;
            let secs = dur.as_secs() % 60;
            info!(
                "  {} potion: slot {:?}, re-drink every {}m{:02}s",
                potion.to_string().magenta(),
                slot,
                mins,
                secs,
            );
        }
    }

    // fish whitelist — resolve every pattern against the constants PHF map.
    let mut matched: Vec<(&str, i32)> = Vec::new();
    for (id, name) in constants::items::ITEM_NAMES.entries() {
        for pattern in &config.fisher.fishes {
            if name.to_lowercase().contains(&pattern.to_lowercase()) {
                matched.push((name, *id));
                break; // avoid duplicates from multiple patterns matching the same fish
            }
        }
    }
    matched.sort_by(|a, b| a.0.cmp(b.0));

    if matched.is_empty() {
        info!("  whitelist: {}", "no matching fish found!".red());
    } else {
        info!("  whitelist ({} fish):", matched.len().to_string().cyan());
        for (name, id) in &matched {
            info!("    - {} (id {})", name.green(), id);
        }
    }

    info!("{}", "══════════════════════════════════════".bright_blue());
    info!(
        "macro is {}, press {:?} to start",
        "OFF".red(),
        config.fisher.hotkeys.toggle,
    );
}

pub struct Fisher {
    config: Config,
    scanner_tx: mpsc::UnboundedSender<ScannerCommand>,
}

impl Fisher {
    pub fn new(config: Config, scanner_tx: mpsc::UnboundedSender<ScannerCommand>) -> Result<Self> {
        Ok(Self { config, scanner_tx })
    }

    fn resolve_fish(fish_id: i32) -> &'static str {
        constants::items::ITEM_NAMES
            .get(&fish_id)
            .copied()
            .unwrap_or("Unknown Fish")
    }

    fn select_slot(&self, slot: Key) -> Result<()> {
        keyboard::press_key(slot)
    }

    fn cast_rod(&self) -> Result<()> {
        self.select_slot(self.config.fisher.rod_slot)?;
        std::thread::sleep(Duration::from_millis(150));
        keyboard::click_mouse()
    }

    fn drink_potion(&self, slot: Key) -> Result<()> {
        self.select_slot(slot)?;
        std::thread::sleep(Duration::from_millis(100));
        keyboard::click_mouse()
    }

    fn catch_and_recast(&self) -> Result<()> {
        keyboard::click_mouse()?;
        std::thread::sleep(Duration::from_millis(self.config.fisher.recast_interval));
        // zero memory to avoid improper catch detection right after a catch
        self.scanner_tx.send(ScannerCommand::ZeroMemory)?;
        self.cast_rod()
    }

    pub async fn run(self, scanner: MemoryScanner) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<FisherEvent>();

        let (mut scan_rx, _scan_handle) = scanner.run().await;
        {
            let tx = tx.clone();
            tokio::spawn(async move {
                while let Ok(fish_id) = scan_rx.recv().await {
                    let _ = tx.send(FisherEvent::FishDetected {
                        fish_id,
                        timestamp: Instant::now(),
                    });
                }
            });
        }

        spawn_hotkey_listener(tx.clone(), &self.config.fisher.hotkeys);

        let mut scheduler = PotionScheduler::new(&self.config, tx.clone());

        print_welcome(&self.config, &scheduler);

        let active = AtomicBool::new(false);
        let paused = AtomicBool::new(false);

        while let Some(event) = rx.recv().await {
            if !active.load(Ordering::Relaxed) {
                match event {
                    FisherEvent::Toggle => {
                        info!("{}", "macro toggled ON".green());
                        active.store(true, Ordering::Relaxed);
                        paused.store(false, Ordering::Relaxed);
                        self.scanner_tx.send(ScannerCommand::TogglePause(true))?;

                        scheduler.schedule_all();
                        if scheduler.is_empty() {
                            self.cast_rod().wrap_err("initial cast")?;
                        }
                    }
                    _ => { /* swallow everything while stopped */ }
                }
                continue;
            }

            match event {
                FisherEvent::Toggle => {
                    info!("{}", "macro toggled OFF — full reset".red());
                    active.store(false, Ordering::Relaxed);
                    paused.store(false, Ordering::Relaxed);
                    self.scanner_tx.send(ScannerCommand::TogglePause(false))?;

                    scheduler.cancel_all();
                }
                FisherEvent::Pause => {
                    let was_paused = paused.fetch_xor(true, Ordering::Relaxed);
                    if was_paused {
                        info!("{}", "macro RESUMED".green());
                        self.scanner_tx.send(ScannerCommand::TogglePause(true))?;

                        // re-drink any potions that expired while paused.
                        scheduler.schedule_all();
                        self.cast_rod().wrap_err("cast after resume")?;
                    } else {
                        info!("{}", "macro PAUSED".yellow());
                        self.scanner_tx.send(ScannerCommand::TogglePause(false))?;
                    }
                }

                FisherEvent::FishDetected { fish_id, timestamp } => {
                    if paused.load(Ordering::Relaxed) {
                        continue;
                    }
                    if timestamp.elapsed() > FISH_EVENT_TTL {
                        debug!("discarding stale fish event (id {})", fish_id);
                        continue;
                    }

                    let fish_name = Self::resolve_fish(fish_id);
                    debug!("detected {} (id {})", fish_name, fish_id);

                    let matches = self
                        .config
                        .fisher
                        .fishes
                        .iter()
                        .any(|f| fish_name.to_lowercase().contains(&f.to_lowercase()));

                    if !matches {
                        debug!("\"{}\" not in configured list, ignoring", fish_name);
                        continue;
                    }

                    info!("caught {} (id {})", fish_name.cyan(), fish_id);
                    std::thread::sleep(Duration::from_millis(self.config.fisher.catch_delay_ms));
                    self.catch_and_recast().wrap_err("catch_and_recast")?;
                }

                FisherEvent::DrinkPotion(potion) => {
                    if paused.load(Ordering::Relaxed) {
                        debug!("ignoring {} potion drink (paused)", potion);
                        continue;
                    }

                    if let Some(slot) = scheduler.slot_for(potion) {
                        info!(
                            "drinking {} potion (slot {:?})",
                            potion.to_string().magenta(),
                            slot,
                        );
                        self.drink_potion(slot)
                            .wrap_err_with(|| format!("drink {} potion", potion))?;
                        scheduler.mark_drunk(potion);

                        // bobber is invalidated after switching slots — recast.
                        std::thread::sleep(Duration::from_millis(
                            self.config.fisher.recast_interval,
                        ));
                        self.cast_rod().wrap_err("recast after potion")?;
                    }
                }

                FisherEvent::Cast { timestamp } => {
                    if timestamp.elapsed() > FISH_EVENT_TTL {
                        debug!("discarding stale cast event");
                        continue;
                    }

                    if !paused.load(Ordering::Relaxed) {
                        self.cast_rod().wrap_err("explicit cast")?;
                    }
                }
            }
        }

        Ok(())
    }
}
