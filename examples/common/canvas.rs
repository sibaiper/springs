//! A tiny software renderer over the `0x00RRGGBB` buffer `minifb` presents.
//!
//! Shared by the examples; each one uses a subset of the API, so unused
//! warnings are switched off here.
#![allow(dead_code)]

use crate::font;

pub struct Canvas {
    width: i32,
    height: i32,
    pixels: Vec<u32>,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    pub fn buffer(&self) -> &[u32] {
        &self.pixels
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Composites `color` over the pixel at (`x`, `y`) with the given coverage.
    pub fn blend(&mut self, x: i32, y: i32, color: u32, coverage: f64) {
        if coverage <= 0.0 || x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }

        let coverage = coverage.min(1.0);
        let index = (y * self.width + x) as usize;
        let destination = self.pixels[index];

        let channel = |shift: u32| {
            let source = ((color >> shift) & 0xFF) as f64;
            let existing = ((destination >> shift) & 0xFF) as f64;
            ((existing + (source - existing) * coverage).round() as u32) << shift
        };

        self.pixels[index] = channel(16) | channel(8) | channel(0);
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        for row in y.max(0)..(y + height).min(self.height) {
            for column in x.max(0)..(x + width).min(self.width) {
                self.pixels[(row * self.width + column) as usize] = color;
            }
        }
    }

    pub fn outline(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        self.rect(x, y, width, 1, color);
        self.rect(x, y + height - 1, width, 1, color);
        self.rect(x, y, 1, height, color);
        self.rect(x + width - 1, y, 1, height, color);
    }

    /// A vertical dashed line, `on` pixels drawn for every `on + off`.
    pub fn dashed_column(&mut self, x: i32, top: i32, bottom: i32, on: i32, off: i32, color: u32) {
        let mut y = top;
        while y < bottom {
            self.rect(x, y, 1, on.min(bottom - y), color);
            y += on + off;
        }
    }

    /// An anti-aliased filled circle.
    pub fn disc(&mut self, cx: f64, cy: f64, radius: f64, color: u32) {
        self.for_each_pixel_near(
            cx,
            cy,
            radius + 1.0,
            |distance| radius + 0.5 - distance,
            color,
        );
    }

    /// An anti-aliased circle outline of the given stroke `weight`.
    pub fn ring(&mut self, cx: f64, cy: f64, radius: f64, weight: f64, color: u32) {
        self.for_each_pixel_near(
            cx,
            cy,
            radius + weight,
            |distance| weight * 0.5 + 0.5 - (distance - radius).abs(),
            color,
        );
    }

    fn for_each_pixel_near(
        &mut self,
        cx: f64,
        cy: f64,
        reach: f64,
        coverage: impl Fn(f64) -> f64,
        color: u32,
    ) {
        let left = (cx - reach).floor() as i32;
        let right = (cx + reach).ceil() as i32;
        let top = (cy - reach).floor() as i32;
        let bottom = (cy + reach).ceil() as i32;

        for y in top..=bottom {
            for x in left..=right {
                let dx = f64::from(x) + 0.5 - cx;
                let dy = f64::from(y) + 0.5 - cy;
                self.blend(x, y, color, coverage(dx.hypot(dy)).clamp(0.0, 1.0));
            }
        }
    }

    /// Draws `text` with its top-left corner at (`x`, `y`), one font pixel per
    /// `scale`×`scale` block. Returns the x the next character would start at.
    pub fn text(&mut self, x: i32, y: i32, scale: i32, color: u32, text: &str) -> i32 {
        let mut cursor = x;

        for character in text.chars() {
            if let Some(rows) = font::glyph(character) {
                for (row, bits) in rows.iter().enumerate() {
                    for column in 0..font::GLYPH_WIDTH {
                        if bits & (1 << (font::GLYPH_WIDTH - 1 - column)) != 0 {
                            let x = cursor + column * scale;
                            let y = y + row as i32 * scale;
                            self.rect(x, y, scale, scale, color);
                        }
                    }
                }
            }
            cursor += font::ADVANCE * scale;
        }

        cursor
    }
}

/// Shapes built on top of the primitives above.
impl Canvas {
    /// An anti-aliased line with rounded ends, of the given stroke `weight`.
    pub fn line(&mut self, from: (f64, f64), to: (f64, f64), weight: f64, color: u32) {
        let (x0, y0) = from;
        let (x1, y1) = to;
        let (dx, dy) = (x1 - x0, y1 - y0);
        let length_squared = dx * dx + dy * dy;

        let reach = weight * 0.5 + 1.0;
        let left = (x0.min(x1) - reach).floor() as i32;
        let right = (x0.max(x1) + reach).ceil() as i32;
        let top = (y0.min(y1) - reach).floor() as i32;
        let bottom = (y0.max(y1) + reach).ceil() as i32;

        for y in top..=bottom {
            for x in left..=right {
                let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);

                // Nearest point on the segment, then distance to it.
                let along = if length_squared > 0.0 {
                    (((px - x0) * dx + (py - y0) * dy) / length_squared).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let distance = (px - (x0 + along * dx)).hypot(py - (y0 + along * dy));

                self.blend(x, y, color, (weight * 0.5 + 0.5 - distance).clamp(0.0, 1.0));
            }
        }
    }

    /// An anti-aliased rounded rectangle, via the usual rounded-box distance
    /// field: distance outside the corners plus distance inside the body.
    pub fn rounded_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
        color: u32,
    ) {
        let radius = radius.min(width * 0.5).min(height * 0.5).max(0.0);
        let (cx, cy) = (x + width * 0.5, y + height * 0.5);
        let (half_width, half_height) = (width * 0.5 - radius, height * 0.5 - radius);

        let left = (x - 1.0).floor() as i32;
        let right = (x + width + 1.0).ceil() as i32;
        let top = (y - 1.0).floor() as i32;
        let bottom = (y + height + 1.0).ceil() as i32;

        for row in top..=bottom {
            for column in left..=right {
                let px = (f64::from(column) + 0.5 - cx).abs() - half_width;
                let py = (f64::from(row) + 0.5 - cy).abs() - half_height;

                let outside = px.max(0.0).hypot(py.max(0.0));
                let inside = px.max(py).min(0.0);
                let distance = outside + inside - radius;

                self.blend(column, row, color, (0.5 - distance).clamp(0.0, 1.0));
            }
        }
    }
}

/// Linear blend between two `0x00RRGGBB` colours.
pub fn mix(from: u32, to: u32, amount: f64) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let start = ((from >> shift) & 0xFF) as f64;
        let end = ((to >> shift) & 0xFF) as f64;
        ((start + (end - start) * amount).round() as u32) << shift
    };

    channel(16) | channel(8) | channel(0)
}
