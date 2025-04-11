use crate::world::World;
use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    event::mouseevent::{MouseEvent, MouseEventListener, MOUSE_EVENT},
    framebuffer::Display,
    world::{Renderable, FRAMEBUFFER, WORLD},
};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
};

pub struct Window {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    changed: bool,
}

impl Window {
    pub fn new(x: i32, y: i32) -> Arc<Mutex<Window>> {
        let window = Arc::new(Mutex::new(Self {
            x,
            y,
            w: 500,
            h: 500,
            changed: true,
        }));
        // MOUSE_EVENT.lock().register_listener(x.clone());
        // WORLD.lock().register(window);

        window
    }

    pub fn get_location(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    pub fn click(&self, x: i32, y: i32) {
        println!("window click {x}, {y}");
    }

    pub fn render(&mut self, _state: &World) {
        if self.changed {
            let mut fb = FRAMEBUFFER.lock();
            let mut display = Display::new(&mut fb);

            let style = PrimitiveStyleBuilder::new()
                .stroke_color(Rgb888::WHITE)
                .stroke_width(3)
                .stroke_alignment(StrokeAlignment::Outside)
                .fill_color(Rgb888::BLACK)
                .build();

            Rectangle::new(Point::new(self.x, self.y), Size::new(self.w, self.h))
                .into_styled(style)
                .draw(&mut display)
                .unwrap();

            Rectangle::new(Point::new(self.x, self.y), Size::new(self.w, 25))
                .into_styled(style)
                .draw(&mut display)
                .unwrap();

            Rectangle::new(
                Point::new(self.x + (self.w - 25) as i32, self.y),
                Size::new(25, 25),
            )
            .into_styled(style)
            .draw(&mut display)
            .unwrap();

            self.changed = false;
        }
    }
}

// impl MouseEventListener for Window {
//     fn handle(&mut self, e: MouseEvent) {
//         self.x = self.x + e.x_delta as i32;
//         self.y = self.y - e.y_delta as i32;
//         if self.x < 0 {
//             self.x = 0;
//         } else if self.x > FRAMEBUFFER.lock().info().width as i32 {
//             self.x = FRAMEBUFFER.lock().info().width as i32;
//         }

//         if self.y < 0 {
//             self.y = 0;
//         } else if self.y > FRAMEBUFFER.lock().info().height as i32 {
//             self.y = FRAMEBUFFER.lock().info().height as i32;
//         }
//         // println!("{:?}", e);
//     }
// }
