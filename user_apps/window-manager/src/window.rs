use crate::framebuffer::Display;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
};

pub struct Window {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Window {
    pub fn new(id: u32, x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { id, x, y, w, h }
    }

    pub fn get_location(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    pub fn click(&self, x: i32, y: i32) {
        println!("window click {x}, {y}");
    }

    pub fn render(&mut self, display: &mut Display) {
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb888::WHITE)
            .stroke_width(3)
            .stroke_alignment(StrokeAlignment::Outside)
            .fill_color(Rgb888::BLACK)
            .build();

        Rectangle::new(Point::new(self.x, self.y), Size::new(self.w, self.h))
            .into_styled(style)
            .draw(display)
            .unwrap();

        Rectangle::new(Point::new(self.x, self.y), Size::new(self.w, 25))
            .into_styled(style)
            .draw(display)
            .unwrap();

        Rectangle::new(
            Point::new(self.x + (self.w - 25) as i32, self.y),
            Size::new(25, 25),
        )
        .into_styled(style)
        .draw(display)
        .unwrap();
    }
}
