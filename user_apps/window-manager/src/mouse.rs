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
    primitives::{PrimitiveStyleBuilder, Rectangle},
};

pub struct Mouse {
    x: i32,
    y: i32,
    last_rendered_x: i32,
    last_rendered_y: i32,
}

impl Mouse {
    pub fn new() {
        let x = Arc::new(Mutex::new(Self {
            x: 0,
            y: 0,
            last_rendered_x: -1,
            last_rendered_y: -1,
        }));
        MOUSE_EVENT.lock().register_listener(x.clone());
        WORLD.lock().register(x);
    }
}

impl MouseEventListener for Mouse {
    fn handle(&mut self, e: MouseEvent) {
        self.x = self.x + e.x_delta as i32;
        self.y = self.y - e.y_delta as i32;
        if self.x < 0 {
            self.x = 0;
        } else if self.x > FRAMEBUFFER.lock().info().width as i32 {
            self.x = FRAMEBUFFER.lock().info().width as i32;
        }

        if self.y < 0 {
            self.y = 0;
        } else if self.y > FRAMEBUFFER.lock().info().height as i32 {
            self.y = FRAMEBUFFER.lock().info().height as i32;
        }
        // println!("{:?}", e);
    }
}

impl Renderable for Mouse {
    fn render(&mut self) {
        if self.last_rendered_x != self.x || self.last_rendered_y != self.y {
            let mut fb = FRAMEBUFFER.lock();
            let mut display = Display::new(&mut fb);

            let white_style = PrimitiveStyleBuilder::new()
                .fill_color(Rgb888::WHITE)
                .build();
            let black_style: embedded_graphics::primitives::PrimitiveStyle<Rgb888> =
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb888::BLACK)
                    .build();
            Rectangle::new(
                Point::new(self.last_rendered_x, self.last_rendered_y),
                Size::new(5, 5),
            )
            .into_styled(black_style)
            .draw(&mut display)
            .unwrap();
            Rectangle::new(Point::new(self.x, self.y), Size::new(5, 5))
                .into_styled(white_style)
                .draw(&mut display)
                .unwrap();
            self.last_rendered_x = self.x;
            self.last_rendered_y = self.y;
        }
    }
}
