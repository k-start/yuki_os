use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::framebuffer::{self, FrameBuffer};

lazy_static! {
    pub static ref FRAMEBUFFER: Mutex<FrameBuffer> = {
        let fd = unsafe { user_api::syscalls::open(b"/framebuffer/0") };
        Mutex::new(framebuffer::FrameBuffer::new(fd))
    };
    pub static ref WORLD: Mutex<World> = Mutex::new(World::new());
}

pub struct World {
    objects: Vec<Arc<Mutex<dyn Renderable + Send>>>,
    pub mouse_x: i32,
    pub mouse_y: i32,
}

impl World {
    fn new() -> Self {
        let mut fb = FRAMEBUFFER.lock();
        fb.clear();

        World {
            objects: Vec::new(),
            mouse_x: 0,
            mouse_y: 0,
        }
    }

    pub fn register(&mut self, listener: Arc<Mutex<dyn Renderable + Send>>) {
        self.objects.push(listener);
    }

    pub fn render(&self) {
        for o in &self.objects {
            o.lock().render(self)
        }
    }
}

pub trait Renderable {
    fn render(&mut self, state: &World);
}
