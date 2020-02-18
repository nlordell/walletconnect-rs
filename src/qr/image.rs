use qrcode::render::{Canvas, Pixel};
use qrcode::types::Color;
use std::iter::{Copied, Zip};
use std::slice::Iter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dot {
    Black,
    White,
}

impl Pixel for Dot {
    type Image = Grid;
    type Canvas = Grid;

    fn default_color(color: Color) -> Self {
        match color {
            Color::Light => Dot::White,
            Color::Dark => Dot::Black,
        }
    }

    fn default_unit_size() -> (u32, u32) {
        (1, 1)
    }
}

#[derive(Clone, Debug)]
pub struct Grid {
    dots: Vec<Dot>,
    width: usize,
    dark: Dot,
}

pub type Row<'a> = Copied<Iter<'a, Dot>>;
pub type Line<'a> = Zip<Row<'a>, Row<'a>>;

impl Grid {
    pub fn row(&self, row: usize) -> Row<'_> {
        let pos = row * self.width;
        self.dots[pos..pos + self.width].iter().copied()
    }

    pub fn line(&self, line: usize) -> Line<'_> {
        let y = line * 2;
        self.row(y).zip(self.row(y + 1))
    }

    pub fn lines(&self) -> (impl Iterator<Item = Line<'_>>, Option<Row<'_>>) {
        let height = self.dots.len() / self.width;
        let count = height / 2;

        let lines = (0..count).map(move |line| self.line(line));
        let last_row = if height % 2 == 1 {
            Some(self.row(height - 1))
        } else {
            None
        };

        (lines, last_row)
    }
}

impl Canvas for Grid {
    type Pixel = Dot;
    type Image = Self;

    fn new(width: u32, height: u32, dark_pixel: Self::Pixel, light_pixel: Self::Pixel) -> Self {
        let (w, h) = (width as usize, height as usize);
        Grid {
            dots: vec![light_pixel; w * h],
            width: w,
            dark: dark_pixel,
        }
    }

    fn draw_dark_pixel(&mut self, x: u32, y: u32) {
        let (x, y) = (x as usize, y as usize);

        let i = x + y * self.width;
        if x >= self.width || i >= self.dots.len() {
            panic!("pixel out of bounds!")
        }

        self.dots[i] = self.dark;
    }

    fn into_image(self) -> Self::Image {
        self
    }
}
