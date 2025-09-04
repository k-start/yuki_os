use ps2_mouse::{Mouse, MouseState};
use spin::Mutex;

pub static MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());

// Initialize the mouse and set the on complete event.
pub fn init_mouse() {
    // crate::fs::vfs::open("/dev/mouse").unwrap();
    MOUSE.lock().init().unwrap();
    MOUSE.lock().set_on_complete(on_complete);
}

// This will be fired when a packet is finished being processed.
fn on_complete(mouse_state: MouseState) {
    // let file = crate::fs::vfs::open("/dev/mouse").unwrap();
    // let button_state =
    //     mouse_state.left_button_down() as u8 + ((mouse_state.right_button_down() as u8) << 1);
    // let buf: [u8; 3] = [
    //     button_state,
    //     mouse_state.get_x() as u8,
    //     mouse_state.get_y() as u8,
    // ];
    // crate::fs::vfs::write(&file, &buf).unwrap();
    // println!("{:?}", mouse_state);
}
