use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::framebuffer::{self, Display, FrameBuffer};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    text::Text,
};

lazy_static! {
    pub static ref FRAMEBUFFER: Mutex<FrameBuffer> = {
        let fd = unsafe { user_api::syscalls::open(b"/framebuffer/0\0") };
        Mutex::new(framebuffer::FrameBuffer::new(fd))
    };
    pub static ref WORLD: Mutex<World> = Mutex::new(World::new());
}

pub struct World {
    objects: Vec<Arc<Mutex<dyn Renderable + Send>>>,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub dirty: bool,
    pub last_render_cycles: u64,
}

impl World {
    fn new() -> Self {
        let mut fb = FRAMEBUFFER.lock();
        fb.clear();

        World {
            objects: Vec::new(),
            mouse_x: 0,
            mouse_y: 0,
            dirty: true,
            last_render_cycles: 0,
        }
    }

    pub fn register(&mut self, listener: Arc<Mutex<dyn Renderable + Send>>) {
        self.objects.push(listener);
    }

    // pub fn render(&mut self) {
    //     if self.dirty {
    //         {
    //             let mut fb = FRAMEBUFFER.lock();
    //             fb.clear();
    //         }

    //         for o in &self.objects {
    //             o.lock().render(self)
    //         }

    //         {
    //             let mut fb = FRAMEBUFFER.lock();
    //             fb.flush();
    //         }

    //         self.dirty = false;
    //     }
    // }

    pub fn render(&mut self) {
        if self.dirty {
            let start_cycles = unsafe { core::arch::x86_64::_rdtsc() };

            {
                let mut fb = FRAMEBUFFER.lock();
                let mut display = Display::new(&mut fb);

                display.clear(Rgb888::new(0, 0, 0)).unwrap();

                // Draw render time text on the status bar
                let text_style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
                let text = alloc::format!(
                    "Render time: {:.2} ms ({} cycles)",
                    (self.last_render_cycles as f64) / 2_000_000.0,
                    self.last_render_cycles
                );
                Text::new(&text, Point::new(10, 16), text_style)
                    .draw(&mut display)
                    .unwrap();
            }

            for o in &self.objects {
                o.lock().render(self);
            }

            {
                let mut fb = FRAMEBUFFER.lock();
                fb.flush();
            }

            self.dirty = false;

            let end_cycles = unsafe { core::arch::x86_64::_rdtsc() };
            self.last_render_cycles = end_cycles - start_cycles;
        }
    }
}

pub trait Renderable {
    fn render(&mut self, state: &World);
}
