use std::path::PathBuf;
use std::sync::Arc;

use easy_config_store::ConfigStore;
use eyre::Result;
use log::{debug, info};
use rdev::Key;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

pub type Config = Arc<ConfigInner>;

// delays and intervals
const DEFAULT_POLL_INTERVAL: u64 = 25;
const DEFAULT_RECAST_INTERVAL: u64 = 1000;
const DEFAULT_CATCH_DELAY: u64 = 250;

// potion durations
const DEFAULT_SONAR_POTION_DURATION_SECS: u64 = 480;
const DEFAULT_FISHING_POTION_DURATION_SECS: u64 = 480;
const DEFAULT_CRATE_POTION_DURATION_SECS: u64 = 240;
/// x min + 5 + 5 where x is the food buff duration and the result is in minutes (multiply by 60 here)
const DEFAULT_FOOD_DURATION_SECS: u64 = 1080;

// hotkeys
const DEFAULT_TOGGLE_HOTKEY: Key = Key::BackSlash;
const DEFAULT_PAUSE_HOTKEY: Key = Key::RightBracket;
const DEFAULT_ROD_HOTKEY: Key = Key::Num1;

pub fn config(path: &PathBuf) -> Result<Config> {
    let config_store = ConfigStore::<ConfigInner>::read(path, "config".to_string())?;
    let inner = (*config_store).clone();
    info!("config parsing successful");
    debug!(
        "loaded configuration:\n{}",
        toml::to_string_pretty(&inner)?.trim()
    );
    Ok(Arc::new(inner))
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ConfigInner {
    #[serde(default)]
    pub scanner: ScannerConfig,
    pub fisher: FisherConfig,
}

#[derive(SmartDefault, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ScannerConfig {
    #[default(DEFAULT_POLL_INTERVAL)]
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
}

#[derive(SmartDefault, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct FisherConfig {
    #[default(DEFAULT_ROD_HOTKEY)]
    #[serde(default = "default_rod_hotkey")]
    pub rod_slot: Key,

    pub fishes: Vec<String>,

    #[default(DEFAULT_RECAST_INTERVAL)]
    #[serde(default = "default_recast_interval")]
    pub recast_interval: u64,

    #[default(DEFAULT_CATCH_DELAY)]
    #[serde(default = "default_catch_delay")]
    pub catch_delay_ms: u64,

    #[serde(default)]
    pub potions: PotionsConfig,

    #[serde(default)]
    pub hotkeys: HotkeyConfig,
}

#[derive(SmartDefault, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct PotionsConfig {
    #[default(DEFAULT_SONAR_POTION_DURATION_SECS)]
    #[serde(default = "default_sonar_potion_duration_secs")]
    pub sonar_potion_duration_secs: u64,
    pub sonar_potion: Option<Key>,

    #[default(DEFAULT_FISHING_POTION_DURATION_SECS)]
    #[serde(default = "default_fishing_potion_duration_secs")]
    pub fishing_potion_duration_secs: u64,
    pub fishing_potion: Option<Key>,

    #[default(DEFAULT_CRATE_POTION_DURATION_SECS)]
    #[serde(default = "default_crate_potion_duration_secs")]
    pub crate_potion_duration_secs: u64,
    pub crate_potion: Option<Key>,

    #[default(DEFAULT_FOOD_DURATION_SECS)]
    #[serde(default = "default_food_duration_secs")]
    pub food_duration_secs: u64,
    pub food: Option<Key>,
}

#[derive(SmartDefault, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct HotkeyConfig {
    #[default(DEFAULT_TOGGLE_HOTKEY)]
    #[serde(default = "default_toggle_hotkey")]
    pub toggle: Key,

    #[default(DEFAULT_PAUSE_HOTKEY)]
    #[serde(default = "default_pause_hotkey")]
    pub pause: Key,
}

impl Default for ConfigInner {
    fn default() -> Self {
        let cfg = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.default.toml",));
        toml::from_str(cfg).unwrap() // should be okay
    }
}

fn default_poll_interval() -> u64 {
    DEFAULT_POLL_INTERVAL
}
fn default_recast_interval() -> u64 {
    DEFAULT_RECAST_INTERVAL
}
fn default_catch_delay() -> u64 {
    DEFAULT_CATCH_DELAY
}
fn default_sonar_potion_duration_secs() -> u64 {
    DEFAULT_SONAR_POTION_DURATION_SECS
}
fn default_fishing_potion_duration_secs() -> u64 {
    DEFAULT_FISHING_POTION_DURATION_SECS
}
fn default_crate_potion_duration_secs() -> u64 {
    DEFAULT_CRATE_POTION_DURATION_SECS
}
fn default_rod_hotkey() -> Key {
    DEFAULT_ROD_HOTKEY
}
fn default_toggle_hotkey() -> Key {
    DEFAULT_TOGGLE_HOTKEY
}
fn default_pause_hotkey() -> Key {
    DEFAULT_PAUSE_HOTKEY
}
fn default_food_duration_secs() -> u64 {
    DEFAULT_FOOD_DURATION_SECS
}
