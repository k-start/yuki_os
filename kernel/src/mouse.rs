use ps2_mouse::{Mouse, MouseState};
use spin::Mutex;

pub static MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());

// Initialize the mouse and set the on complete event.
pub fn init_mouse() {
    MOUSE.lock().init().unwrap();
    MOUSE.lock().set_on_complete(on_complete);
}

// This will be fired when a packet is finished being processed.
fn on_complete(mouse_state: MouseState) {
    println!("{:?}", mouse_state);
}
