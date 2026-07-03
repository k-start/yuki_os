use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref MOUSE_EVENT: Mutex<MouseEventHandler> = Mutex::new(MouseEventHandler::new());
}

pub struct MouseEventHandler {
    fd: usize,
}

impl MouseEventHandler {
    pub fn new() -> Self {
        let fd = unsafe { user_api::syscalls::open(b"/dev/mouse\0") };
        MouseEventHandler { fd }
    }

    pub fn poll(&self) -> Option<MouseEvent> {
        let mut mouse_buf: [u8; 3] = [0; 3];
        let bytes_read = unsafe { user_api::syscalls::read(self.fd, &mut mouse_buf) };

        let _ = mouse_buf == [0; 3]; // Fix me - weird bug where without this bytes_read = 0 even if they are read

        if bytes_read <= 0 {
            return None;
        }

        Some(MouseEvent {
            x_delta: mouse_buf[1] as i8,
            y_delta: mouse_buf[2] as i8,
            left: (mouse_buf[0] & 0x1) != 0,
            right: (mouse_buf[0] & 0x2) != 0,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MouseEvent {
    pub x_delta: i8,
    pub y_delta: i8,
    pub left: bool,
    pub right: bool,
}

