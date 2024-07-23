use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref MOUSE_EVENT: Mutex<MouseEventHandler> = Mutex::new(MouseEventHandler::new());
}

pub struct MouseEventHandler {
    fd: usize,
    listeners: Vec<Arc<Mutex<dyn MouseEventListener + Send>>>,
}

impl MouseEventHandler {
    pub fn new() -> Self {
        let fd = unsafe { user_api::syscalls::open(b"/dev/mouse") };
        MouseEventHandler {
            fd,
            listeners: Vec::new(),
        }
    }

    pub fn poll(&self) {
        let mut mouse_buf: [u8; 3] = [0; 3];
        let bytes_read = unsafe { user_api::syscalls::read(self.fd, &mut mouse_buf) };

        let _ = mouse_buf == [0; 3]; // Fix me - weird bug where without this bytes_read = 0 even if they are read

        if bytes_read > 0 {
            // println!("mouse bytes_read = {bytes_read}");
            // let x_delta = mouse_buf[1] as i8;
            // let y_delta = mouse_buf[2] as i8;

            // println!(
            //     "x={x_delta}, y={y_delta}, left = {}, right = {}",
            //     (mouse_buf[0] & 0x1) != 0,
            //     (mouse_buf[0] & 0x2) != 0
            // );
            let e = MouseEvent {
                x_delta: mouse_buf[1] as i8,
                y_delta: mouse_buf[2] as i8,
                left: (mouse_buf[0] & 0x1) != 0,
                right: (mouse_buf[0] & 0x2) != 0,
            };

            for listener in &self.listeners {
                listener.lock().handle(e.clone());
            }
        }
    }

    pub fn register_listener(&mut self, listener: Arc<Mutex<dyn MouseEventListener + Send>>) {
        self.listeners.push(listener);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MouseEvent {
    pub x_delta: i8,
    pub y_delta: i8,
    left: bool,
    right: bool,
}

pub trait MouseEventListener {
    fn handle(&mut self, e: MouseEvent);
}
