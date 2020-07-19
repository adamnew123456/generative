use std::io;

/// A simple RGB color. Does not support an alpha because the underlying
/// Framebuffer is not capable of performing blending.
#[derive(Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    alpha: u8,
}

impl Color {
    /// Creates a new color from R, G and B components
    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            r,
            g,
            b,
            alpha: 255,
        }
    }

    /// Creates a new color from R, G and B components
    pub fn rgba(r: u8, g: u8, b: u8, alpha: u8) -> Color {
        Color { r, g, b, alpha }
    }

    /// Returns a Color representing pure white
    pub fn white() -> Color {
        Color::rgb(255, 255, 255)
    }

    /// Returns a Color representing pure black
    pub fn black() -> Color {
        Color::rgb(0, 0, 0)
    }

    /// Writes the contents of the color to a raw 24-bit image buffer at a given
    /// offset, in RGB order
    fn write(&self, bitmap: &mut Vec<u8>, offset: usize) {
        if self.alpha == 0 {
            return;
        } else if self.alpha == 255 {
            bitmap[offset] = self.r;
            bitmap[offset + 1] = self.g;
            bitmap[offset + 2] = self.b;
        } else {
            let base_blend = (255 - self.alpha) as u16;
            let (r, g, b) = {
                let blend = |offset, color| {
                    ((bitmap[offset] as u16 * base_blend) + (color as u16 * self.alpha as u16))
                        / 255
                };
                (
                    blend(offset, self.r) as u8,
                    blend(offset + 1, self.g) as u8,
                    blend(offset + 2, self.b) as u8,
                )
            };

            bitmap[offset] = r;
            bitmap[offset + 1] = g;
            bitmap[offset + 2] = b;
        }
    }
}

/// An array of raw values which allows for basic batch processing on a
/// Framebuffer
pub struct Maskbuffer {
    mask: Vec<u8>,
    width: u32,
    height: u32,
}

impl Maskbuffer {
    pub fn new(width: u32, height: u32, full: u8) -> Maskbuffer {
        let mut mask = Vec::with_capacity((width * height) as usize);
        mask.resize((width * height) as usize, 0);

        let mut fb = Maskbuffer {
            mask,
            width,
            height,
        };

        fb.fill(full);
        fb
    }

    /// Overwrites the entire maskbuffer with the given color
    pub fn fill(&mut self, value: u8) {
        for pixel in 0..self.mask.len() {
            self.mask[pixel] = value;
        }
    }

    /// Updates the value of the maskbuffer at the given location
    pub fn update<T: Fn(u8) -> u8>(&mut self, x: i64, y: i64, func: T) {
        if x < 0 || x >= self.width as i64 {
            return;
        }

        if y < 0 || y >= self.height as i64 {
            return;
        }

        let offset = (y * self.width as i64) + x;
        self.mask[offset as usize] = func(self.mask[offset as usize]);
    }
}

/// An array of pixels with fixed dimensions
pub struct Framebuffer {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// Writes all the contents of the buffer to the output stream, breaking down
/// the buffer into chunks as necessary
fn write_all<T: io::Write>(output: &mut T, buffer: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        let size = output.write(&buffer[offset..])?;
        if size == 0 {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        offset += size;
    }

    Ok(())
}

impl Framebuffer {
    /// The framebuffer's bitmap uses 1 byte per channel, with 3 channels
    const BYTES_PER_PIXEL: i64 = 3;

    /// Creates a new Framebuffer of the given size, filled with the given color
    pub fn new(width: u32, height: u32, fill: Color) -> Framebuffer {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        pixels.resize((width * height * 3) as usize, 0);

        let mut fb = Framebuffer {
            pixels,
            width,
            height,
        };

        fb.fill(fill);
        fb
    }

    /// Applies the given maskbuffer to this buffer, updating the value of each
    /// pixel according to the given function and value in the mask
    pub fn mask<T: Fn(u8, (u8, u8, u8)) -> (u8, u8, u8)>(&mut self, mask: &Maskbuffer, func: T) {
        for pixel in 0..(self.width * self.height) {
            let index = pixel as usize;
            let (r, g, b) = func(
                mask.mask[index],
                (
                    self.pixels[index * 3],
                    self.pixels[index * 3 + 1],
                    self.pixels[index * 3 + 2],
                ),
            );
            self.pixels[index * 3] = r;
            self.pixels[index * 3 + 1] = g;
            self.pixels[index * 3 + 2] = b;
        }
    }

    /// Overwrites the entire framebuffer with the given color
    pub fn fill(&mut self, fill: Color) {
        for pixel in 0..(self.width * self.height) {
            fill.write(&mut self.pixels, (3 * pixel) as usize);
        }
    }

    /// Overwrites a region of the framebuffer with the given color
    pub fn fill_rect(&mut self, x: i64, y: i64, width: i64, height: i64, fill: Color) {
        for px in x..(x + width) {
            for py in y..(y + height) {
                self.point_at(px, py, fill);
            }
        }
    }

    /// Draws a single colored pixel on the framebuffer
    pub fn point_at(&mut self, x: i64, y: i64, stroke: Color) {
        if x >= 0 && x < (self.width as i64) && y >= 0 && y < (self.height as i64) {
            let offset = (y * (self.width as i64) * Framebuffer::BYTES_PER_PIXEL)
                + (x * Framebuffer::BYTES_PER_PIXEL);

            stroke.write(&mut self.pixels, offset as usize);
        }
    }

    /// Draws a colored line between two points
    pub fn line_at(&mut self, x: i64, y: i64, x2: i64, y2: i64, stroke: Color) {
        let rise = y2 - y;
        let run = x2 - x;

        if run == 0 {
            // Lines along either axis don't require tracking the slope, since
            // we can hold one coordinate fixed and just draw with the other
            let bottom_y = y.min(y2);
            let top_y = y.max(y2);

            for py in bottom_y..top_y {
                self.point_at(x, py, stroke);
            }
        } else if rise == 0 {
            let left_x = x.min(x2);
            let right_x = x.max(x2);

            for px in left_x..right_x {
                self.point_at(px, y, stroke);
            }
        } else if run.abs() > rise.abs() {
            // Use Bresenham's algorithm otherwise, picking whatever axis moves
            // the most for our basis axis and the other as an error axis
            let left_x = x.min(x2);
            let right_x = x.max(x2);
            let slope = rise as f64 / run as f64;

            let (start_y, sign) = if left_x == x2 {
                (y2, -rise.signum())
            } else {
                (y, rise.signum())
            };

            let error_incr = slope.abs();
            let mut error = 0.0;

            let mut py = start_y;
            for px in left_x..right_x {
                self.point_at(px, py, stroke);
                error += error_incr;
                if error > 0.5 {
                    py += sign;
                    error -= 1.0;
                }
            }
        } else {
            let bottom_y = y.min(y2);
            let top_y = y.max(y2);
            let slope = run as f64 / rise as f64;

            let (start_x, sign) = if bottom_y == y2 {
                (x2, -run.signum())
            } else {
                (x, run.signum())
            };

            let error_incr = slope.abs();
            let mut error = 0.0;

            let mut px = start_x;
            for py in bottom_y..top_y {
                self.point_at(px, py, stroke);
                error += error_incr;
                if error > 0.5 {
                    px += sign;
                    error -= 1.0;
                }
            }
        }
    }

    /// Dumps the framebuffer as a binary PPM image
    pub fn write(&self, output: &mut impl io::Write) -> io::Result<()> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        write_all(output, header.as_bytes())?;
        write_all(output, &self.pixels)
    }
}
