use crate::window::Window;
use alloc::vec::Vec;
use user_api::framebuffer::Display;

pub struct WindowManager {
    pub windows: Vec<Window>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    pub fn add_window(&mut self, window: Window) {
        self.windows.push(window);
    }

    pub fn handle_mouse(&mut self, mouse_x: i32, mouse_y: i32, mouse_left: bool) {
        if mouse_left {
            let mut clicked_idx = None;
            for (idx, window) in self.windows.iter().enumerate().rev() {
                let (w_x, w_y, w_w, w_h) = window.get_location();
                if mouse_x >= w_x
                    && mouse_x <= w_x + w_w as i32
                    && mouse_y >= w_y
                    && mouse_y <= w_y + w_h as i32
                {
                    clicked_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = clicked_idx {
                let window = self.windows.get(idx).unwrap();
                window.click(
                    mouse_x - window.get_location().0,
                    mouse_y - window.get_location().1,
                );
            }
            println!("{mouse_x} {mouse_y} click");
        }
    }

    pub fn render(&mut self, display: &mut Display) {
        for window in &mut self.windows {
            window.render(display);
        }
    }
}
