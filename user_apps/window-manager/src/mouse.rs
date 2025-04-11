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
    primitives::{PrimitiveStyleBuilder, Rectangle},
};

pub struct Mouse {
    last_rendered_x: i32,
    last_rendered_y: i32,
}

impl Mouse {
    pub fn new() {
        let x = Arc::new(Mutex::new(Self {
            last_rendered_x: -1,
            last_rendered_y: -1,
        }));
        MOUSE_EVENT.lock().register_listener(x.clone());
        WORLD.lock().register(x);
    }
}

impl MouseEventListener for Mouse {
    fn handle(&mut self, e: MouseEvent) {
        let mut world = WORLD.lock();
        world.mouse_x += e.x_delta as i32;
        world.mouse_y -= e.y_delta as i32;

        if world.mouse_x < 0 {
            world.mouse_x = 0;
        } else if world.mouse_x > FRAMEBUFFER.lock().info().width as i32 {
            world.mouse_x = FRAMEBUFFER.lock().info().width as i32;
        }

        if world.mouse_y < 0 {
            world.mouse_y = 0;
        } else if world.mouse_y > FRAMEBUFFER.lock().info().height as i32 {
            world.mouse_y = FRAMEBUFFER.lock().info().height as i32;
        }
        // println!("{:?}", e);
    }
}

impl Renderable for Mouse {
    fn render(&mut self, state: &World) {
        if self.last_rendered_x != state.mouse_x || self.last_rendered_y != state.mouse_y {
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
            Rectangle::new(Point::new(state.mouse_x, state.mouse_y), Size::new(5, 5))
                .into_styled(white_style)
                .draw(&mut display)
                .unwrap();
            self.last_rendered_x = state.mouse_x;
            self.last_rendered_y = state.mouse_y;
        }
    }
}
