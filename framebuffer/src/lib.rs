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
        /*
        Ref: http://members.chello.at/~easyfilter/Bresenham.pdf, p.13

        The idea behind this algorithm and the related ones in the paper is to
        figure out the next best pixel incrementally by determining which next
        pixel would be farthest from the line we want to draw. Since we're
        restricted to a grid of pixels each choice creates some deviation
        from the actual line that we want to minimize.

        The line itself is this equation. The error value E for (px, py) is the
        value of the right-hand side. 0 indicates a point directly on the line,
        anything else will be at some offset.

        let dx = x2 - x,
            dy = y2 - y

           py = (px - x) * dy / dx + y
        ~> 0 = (py - y) * dx - (px - x) * dy

        Assuming we're at (px, py) already, we have to figure out what pixel to
        color next. There are three ways we can go (assuming we're in quadrant
        1):

        - px + 1, py + 1:
           (py + 1 - y) * dx - (px + 1 - x) * dy
        ~> E' = dx - dy + E

        - px + 1, py:
           (py - y) * dx - (px + 1 - x) * dy
        ~> E' = -dy + E

        - px, py + 1:
           (py + 1 - y) * dx - (px - x) * dy
        ~> E' = dx + E

        Whatever next point produces the smallest error is the one we want to
        pick.
         */
        let deltax = (x2 - x).abs();
        let stepx = (x2 - x).signum();

        let deltay = -(y2 - y).abs();
        let stepy = (y2 - y).signum();

        let mut error = deltax + deltay;

        let mut px = x;
        let mut py = y;
        loop {
            self.point_at(px, py, stroke);

            let next_error = 2 * error;
            if next_error >= deltay {
                if px == x2 {
                    break;
                }

                error += deltay;
                px += stepx;
            }

            if next_error <= deltax {
                if py == y2 {
                    break;
                }

                error += deltax;
                py += stepy;
            }
        }
    }

    /// Draws a colored circle around the given point
    pub fn circle_at(&mut self, x: i64, y: i64, r: i64, stroke: Color) {
        /*
        Derivation, assuming that x and y are the origin (the offset can be done
        later):

        0 = px^2 + py^2 - r^2

        Assuming that we're in the second quadrant, where the slope of the curve
        is positive:

        - px + 1, py + 1
           (px + 1)^2 + (py + 1)^2 - r^2
        ~> E' = 2*px + 2*py + 2 + E

        - px + 1, py
           (px + 1)^2 + py^2 - r^2
        ~> E' = 2*px + 1 + E

        - px, py + 1
           px^2 + (py + 1)^2 - r^2
        ~> E' = 2*py + 1 + E

        The starting error is for (-r, 0):

        E_1 = (-r + 1)^2 + 1^2 - r^2
        ~>    -2r + 2
         */
        let mut error = -2 * r + 2;

        let mut relx = -r;
        let mut rely = 0;

        while relx <= 0 {
            self.point_at(x + relx, y + rely, stroke); // Quadrant II
            self.point_at(x - relx, y + rely, stroke); // Quadrant I
            self.point_at(x + relx, y - rely, stroke); // Quadrant IV
            self.point_at(x - relx, y - rely, stroke); // Quadrant III

            let next_error = 2 * error;
            if next_error >= 2 * relx + 1 {
                relx += 1;
                error += 2 * relx + 1;
            }

            if next_error <= 2 * rely + 1 {
                rely += 1;
                error += 2 * rely + 1;
            }
        }
    }

    /// Fills a colored circle around the given point
    pub fn circle_fill(&mut self, x: i64, y: i64, r: i64, stroke: Color) {
        let mut error = -2 * r + 2;

        let mut relx = -r;
        let mut rely = 0;

        while relx <= 0 {
            let next_error = 2 * error;
            if next_error >= 2 * relx + 1 {
                relx += 1;
                error += 2 * relx + 1;
            }

            if next_error <= 2 * rely + 1 {
                self.line_at(x + relx, y + rely, x - relx, y + rely, stroke);
                self.line_at(x + relx, y - rely, x - relx, y - rely, stroke);

                rely += 1;
                error += 2 * rely + 1;
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
