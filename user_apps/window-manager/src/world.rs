use alloc::format;
use lazy_static::lazy_static;
use spin::Mutex;
use user_api::window::{CreateResponse, WindowCommand};

use crate::event::mouseevent::MouseEvent;
use crate::framebuffer::{self, Display, FrameBuffer};
use crate::window::Window;
use crate::windowmanager::WindowManager;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
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
    pub window_manager: WindowManager,
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
            window_manager: WindowManager::new(),
            mouse_x: 0,
            mouse_y: 0,
            dirty: true,
            last_render_cycles: 0,
        }
    }

    pub fn handle_mouse_event(&mut self, e: MouseEvent) {
        let old_x = self.mouse_x;
        let old_y = self.mouse_y;

        self.mouse_x += e.x_delta as i32;
        self.mouse_y -= e.y_delta as i32;

        let fb_width = FRAMEBUFFER.lock().info().width as i32;
        let fb_height = FRAMEBUFFER.lock().info().height as i32;

        if self.mouse_x < 0 {
            self.mouse_x = 0;
        } else if self.mouse_x > fb_width {
            self.mouse_x = fb_width;
        }

        if self.mouse_y < 0 {
            self.mouse_y = 0;
        } else if self.mouse_y > fb_height {
            self.mouse_y = fb_height;
        }

        if old_x != self.mouse_x || old_y != self.mouse_y || e.left {
            self.dirty = true;
        }

        self.window_manager
            .handle_mouse(self.mouse_x, self.mouse_y, e.left);
    }

    pub fn handle_ipc(&mut self, cmd: WindowCommand) {
        match cmd {
            WindowCommand::Create(create_request) => {
                println!(
                    "Window CreateRequest: x={}, y={}, width={}, height={}, pid={}",
                    create_request.x,
                    create_request.y,
                    create_request.width,
                    create_request.height,
                    create_request.pid
                );

                let window_id = create_request.pid; // Use pid as window id for now - one day this will be useful

                let new_window = Window::new(
                    window_id,
                    create_request.x,
                    create_request.y,
                    create_request.width,
                    create_request.height,
                );
                self.window_manager.add_window(new_window);

                let resp_fd = unsafe {
                    user_api::syscalls::open(
                        format!("/dev/wm_response_{}\0", create_request.pid).as_bytes(),
                    )
                };

                let response = CreateResponse {
                    window_id,
                    buffer_size: 0, // we will make a proper buffer soon
                };
                let response_slice = unsafe {
                    core::slice::from_raw_parts(
                        &response as *const CreateResponse as *const u8,
                        core::mem::size_of::<CreateResponse>(),
                    )
                };
                unsafe { user_api::syscalls::write(resp_fd, response_slice) };

                self.dirty = true;
            }
        }
    }

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

                // Draw windows
                self.window_manager.render(&mut display);

                // Draw mouse cursor
                let white_style = PrimitiveStyleBuilder::new()
                    .fill_color(Rgb888::WHITE)
                    .build();

                Rectangle::new(Point::new(self.mouse_x, self.mouse_y), Size::new(5, 5))
                    .into_styled(white_style)
                    .draw(&mut display)
                    .unwrap();

                fb.flush();
            }

            self.dirty = false;

            let end_cycles = unsafe { core::arch::x86_64::_rdtsc() };
            self.last_render_cycles = end_cycles - start_cycles;
        }
    }
}
