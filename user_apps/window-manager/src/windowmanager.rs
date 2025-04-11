use crate::window::Window;
use crate::world::World;
use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;

use crate::{
    event::mouseevent::{MouseEvent, MouseEventListener, MOUSE_EVENT},
    world::{Renderable, WORLD},
};

pub struct WindowManager {
    pub windows: Vec<Arc<Mutex<Window>>>,
}

impl WindowManager {
    pub fn new() {
        let mut windows = Vec::new();
        windows.push(Window::new(100, 100));
        windows.push(Window::new(650, 100));
        let windowmanager = Arc::new(Mutex::new(WindowManager { windows }));

        MOUSE_EVENT.lock().register_listener(windowmanager.clone());
        WORLD.lock().register(windowmanager);
    }
}

impl MouseEventListener for WindowManager {
    fn handle(&mut self, e: MouseEvent) {
        if e.left {
            let x = WORLD.lock().mouse_x;
            let y = WORLD.lock().mouse_y;

            // for i in self.windows.clone().into_iter() {
            // let (w_x, w_y, w_w, w_h) = i.lock().get_location();

            // if x >= w_x && x <= w_x + w_w as i32 && y >= w_y && y <= w_y + w_h as i32 {
            //     i.lock().click(x - w_x, y - w_y);
            //     // break;
            // }
            // }

            println!("{x} {y} click");
        }
    }
}

impl Renderable for WindowManager {
    fn render(&mut self, state: &World) {
        for i in self.windows.clone().into_iter() {
            i.lock().render(state);
        }
    }
}
