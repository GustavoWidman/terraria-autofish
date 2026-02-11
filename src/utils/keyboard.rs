use std::time::Duration;

use eyre::Result;
use rdev::{Button, EventType, Key, simulate};

pub const KEY_DELAY: Duration = Duration::from_millis(30);

pub fn send_event(et: EventType) -> Result<()> {
    simulate(&et).map_err(|e| eyre::eyre!("rdev simulate error: {:?}", e))?;
    std::thread::sleep(KEY_DELAY);
    Ok(())
}

pub fn press_key(key: Key) -> Result<()> {
    send_event(EventType::KeyPress(key))?;
    send_event(EventType::KeyRelease(key))
}

pub fn click_mouse() -> Result<()> {
    send_event(EventType::ButtonPress(Button::Left))?;
    send_event(EventType::ButtonRelease(Button::Left))
}
