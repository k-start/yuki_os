use alloc::{sync::Arc, vec::Vec};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, StrokeAlignment},
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::framebuffer::{self, Display, FrameBuffer};

lazy_static! {
    pub static ref FRAMEBUFFER: Mutex<FrameBuffer> = {
        let fd = unsafe { user_api::syscalls::open(b"/framebuffer/0") };
        Mutex::new(framebuffer::FrameBuffer::new(fd))
    };
    pub static ref WORLD: Mutex<World> = Mutex::new(World::new());
}

pub struct World {
    objects: Vec<Arc<Mutex<dyn Renderable + Send>>>,
}

impl World {
    fn new() -> Self {
        let mut fb = FRAMEBUFFER.lock();
        fb.clear();
        let mut display = Display::new(&mut fb);

        let border_stroke = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb888::WHITE)
            .stroke_width(3)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();

        display
            .bounding_box()
            .into_styled(border_stroke)
            .draw(&mut display)
            .unwrap();

        World {
            objects: Vec::new(),
        }
    }

    pub fn register(&mut self, listener: Arc<Mutex<dyn Renderable + Send>>) {
        self.objects.push(listener);
    }

    pub fn render(&self) {
        for o in &self.objects {
            o.lock().render()
        }
    }
}

pub trait Renderable {
    fn render(&mut self);
}
