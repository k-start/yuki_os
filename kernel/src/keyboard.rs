use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

use crate::scheduler;

static KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = {
    Mutex::new(Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    ))
};

pub fn handle_key(scancode: u8) {
    let mut keyboard = KEYBOARD.lock();

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    scheduler::SCHEDULER.read().push_stdin(character as u8)
                }
                DecodedKey::RawKey(_key) => {}
            }
        }
    }
}
