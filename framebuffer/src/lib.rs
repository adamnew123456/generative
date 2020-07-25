use std::io;

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

/// A simple RGB color with transparency.
#[derive(Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub alpha: u8,
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

    /// Creates a new color from R, G, B and transparency components
    pub fn rgba(r: u8, g: u8, b: u8, alpha: u8) -> Color {
        Color { r, g, b, alpha }
    }

    /// Creates a new color based on this color, including only the selected
    /// channels
    pub fn mask(&self, r: bool, g: bool, b: bool) -> Color {
        Color {
            r: if r { self.r } else { 0 },
            g: if g { self.g } else { 0 },
            b: if b { self.b } else { 0 },
            alpha: self.alpha,
        }
    }

    /// Creates a new color from alpha blending this color and the other color,
    /// ignoring this color's alpha if it is present. The output color's alpha
    /// is the same as this alpha.
    pub fn blend(&self, other: Color) -> Color {
        let base_blend = (255 - other.alpha) as u16;

        let (r, g, b) = {
            let blend = |a, b| ((a as u16 * base_blend) + (b as u16 * other.alpha as u16)) / 255;
            (
                blend(self.r, other.r) as u8,
                blend(self.g, other.g) as u8,
                blend(self.b, other.b) as u8,
            )
        };

        Color::rgba(r, g, b, self.alpha)
    }

    /// Returns a Color representing pure white
    pub fn white() -> Color {
        Color::rgb(255, 255, 255)
    }

    /// Returns a Color representing pure black
    pub fn black() -> Color {
        Color::rgb(0, 0, 0)
    }
}

pub trait GraphicBuffer<T: Copy> {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn put_point(&mut self, x: i64, y: i64, color: T);
    fn get_point(&self, x: i64, y: i64) -> Option<T>;
}

/// A graphical buffer containing pixel colors
pub struct FrameBuffer {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl FrameBuffer {
    /// Creates a new FrameBuffer with a black background
    pub fn new(width: u32, height: u32) -> FrameBuffer {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        pixels.resize((width * height * 3) as usize, 0);
        FrameBuffer {
            pixels,
            width,
            height,
        }
    }

    /// Dumps the framebuffer as a binary PPM image
    pub fn write(&self, output: &mut impl io::Write) -> io::Result<()> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        write_all(output, header.as_bytes())?;
        write_all(output, &self.pixels)
    }
}

impl GraphicBuffer<Color> for FrameBuffer {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn get_point(&self, x: i64, y: i64) -> Option<Color> {
        if x < 0 || x >= self.width as i64 {
            None
        } else if y < 0 || x >= self.height as i64 {
            None
        } else {
            let offset = ((y * (self.width as i64) * 3) + (x * 3)) as usize;
            Some(Color::rgb(
                self.pixels[offset],
                self.pixels[offset + 1],
                self.pixels[offset + 2],
            ))
        }
    }

    fn put_point(&mut self, x: i64, y: i64, color: Color) {
        if x < 0 || x >= self.width as i64 {
            return;
        } else if y < 0 || y >= self.height as i64 {
            return;
        } else if color.alpha == 0 {
            return;
        } else if color.alpha == 255 {
            let offset = ((y * (self.width as i64) * 3) + (x * 3)) as usize;
            self.pixels[offset] = color.r;
            self.pixels[offset + 1] = color.g;
            self.pixels[offset + 2] = color.b;
        } else {
            let base_blend = (255 - color.alpha) as u16;
            let offset = ((y * (self.width as i64) * 3) + (x * 3)) as usize;

            let (r, g, b) = {
                let blend = |offset, channel| {
                    ((self.pixels[offset] as u16 * base_blend)
                        + (channel as u16 * color.alpha as u16))
                        / 255
                };
                (
                    blend(offset, color.r) as u8,
                    blend(offset + 1, color.g) as u8,
                    blend(offset + 2, color.b) as u8,
                )
            };

            self.pixels[offset] = r;
            self.pixels[offset + 1] = g;
            self.pixels[offset + 2] = b;
        }
    }
}

/// A masking buffer containing simple integers
pub struct StencilBuffer {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl StencilBuffer {
    /// Creates a new StencilBuffer with a 0 background
    pub fn new(width: u32, height: u32) -> StencilBuffer {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        pixels.resize((width * height) as usize, 0);
        StencilBuffer {
            pixels,
            width,
            height,
        }
    }
}

impl GraphicBuffer<u8> for StencilBuffer {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn get_point(&self, x: i64, y: i64) -> Option<u8> {
        if x < 0 || x >= self.width as i64 {
            None
        } else if y < 0 || x >= self.height as i64 {
            None
        } else {
            let offset = ((y * (self.width as i64)) + x) as usize;
            Some(self.pixels[offset])
        }
    }

    fn put_point(&mut self, x: i64, y: i64, color: u8) {
        if x < 0 || x >= self.width as i64 {
            return;
        } else if y < 0 || x >= self.height as i64 {
            return;
        } else {
            let offset = ((y * (self.width as i64)) + x) as usize;
            self.pixels[offset] = color;
        }
    }
}

/// Performs drawing operations on an underlying graphical buffer
pub struct Canvas<Element: Copy, Buffer: GraphicBuffer<Element>> {
    buffer: Buffer,
    fill: Element,
    stroke: Element,
}

impl<Element: Copy, Buffer: GraphicBuffer<Element>> Canvas<Element, Buffer> {
    /// Initializes a canvas on top of the given buffer with the given fill and
    /// stroke colors
    pub fn new(buffer: Buffer, fill: Element, stroke: Element) -> Canvas<Element, Buffer> {
        Canvas {
            buffer,
            fill,
            stroke,
        }
    }

    /// Gets the underlying buffer for the canvas
    pub fn buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Gets the width of the underlying buffer
    pub fn width(&self) -> u32 {
        self.buffer.width()
    }

    /// Gets the height of the underlying buffer
    pub fn height(&self) -> u32 {
        self.buffer.height()
    }

    /// Gets the given point from the underlying canvas
    pub fn get_point(&self, x: i64, y: i64) -> Option<Element> {
        self.buffer.get_point(x, y)
    }

    /// Puts the given point from the underlying canvas
    pub fn put_point(&mut self, x: i64, y: i64, color: Element) {
        self.buffer.put_point(x, y, color)
    }

    /// Applies a mask function from the other buffer onto this canvas's buffer
    pub fn mask<MaskElement, MaskBuffer, F>(
        &mut self,
        other: &Canvas<MaskElement, MaskBuffer>,
        func: F,
    ) where
        MaskElement: Copy,
        MaskBuffer: GraphicBuffer<MaskElement>,
        F: Fn(Element, MaskElement) -> Element,
    {
        if self.buffer.width() != other.width() {
            return;
        }

        if self.buffer.height() != other.height() {
            return;
        }

        for py in 0..self.buffer.height() {
            for px in 0..self.buffer.width() {
                let src = match self.buffer.get_point(px as i64, py as i64) {
                    None => continue,
                    Some(color) => color,
                };

                let mask = match other.get_point(px as i64, py as i64) {
                    None => continue,
                    Some(color) => color,
                };

                let dest = func(src, mask);
                self.buffer.put_point(px as i64, py as i64, dest);
            }
        }
    }

    /// Sets the current fill color
    pub fn set_fill(&mut self, fill: Element) {
        self.fill = fill;
    }

    /// Sets the current stroke color
    pub fn set_stroke(&mut self, stroke: Element) {
        self.stroke = stroke;
    }

    /// Draws a single pixel at the given point using the current fill
    pub fn fill_point(&mut self, x: i64, y: i64) {
        self.buffer.put_point(x, y, self.fill);
    }

    /// Draws a single pixel at the given point using the current stroke
    pub fn stroke_point(&mut self, x: i64, y: i64) {
        self.buffer.put_point(x, y, self.stroke);
    }

    /// Fills the entire buffer using the currently assigned fill value
    pub fn fill(&mut self) {
        for y in 0..self.buffer.height() {
            for x in 0..self.buffer.width() {
                self.fill_point(x as i64, y as i64);
            }
        }
    }

    /// Fills the given region of the framebuffer with the current fill color
    pub fn fill_rect(&mut self, x: i64, y: i64, width: i64, height: i64) {
        for py in y..(y + height) {
            for px in x..(x + width) {
                self.fill_point(px, py);
            }
        }
    }

    /// Fills the given region of the framebuffer with the given gradient(xratio, yratio)
    pub fn gfill_rect<F>(&mut self, x: i64, y: i64, width: i64, height: i64, gradient: F)
    where
        F: Fn(f64, f64) -> Element,
    {
        for py in y..(y + height) {
            let yratio = (py - y) as f64 / height as f64;
            for px in x..(x + width) {
                let xratio = (px - x) as f64 / width as f64;
                self.buffer.put_point(px, py, gradient(xratio, yratio));
            }
        }
    }

    /// Draws a border around the given region of the framebuffer with the
    /// current stroke color
    pub fn stroke_rect(&mut self, x: i64, y: i64, width: i64, height: i64) {
        for py in y..(y + height) {
            if py == y || y == (y + height) - 1 {
                for px in x..(x + width) {
                    self.stroke_point(py, px);
                }
            } else {
                self.stroke_point(py, x);
                self.stroke_point(py, x + width - 1);
            }
        }
    }

    /// Draws a border around the given region of the framebuffer with the
    /// given gradient(xratio, yratio)
    pub fn gstroke_rect<F>(&mut self, x: i64, y: i64, width: i64, height: i64, gradient: F)
    where
        F: Fn(f64, f64) -> Element,
    {
        for py in y..(y + height) {
            let yratio = (py - y) as f64 / height as f64;
            if py == y || y == (y + height) - 1 {
                for px in x..(x + width) {
                    let xratio = (px - x) as f64 / width as f64;
                    self.buffer.put_point(py, px, gradient(xratio, yratio));
                }
            } else {
                self.buffer.put_point(py, x, gradient(0.0, yratio));
                self.buffer
                    .put_point(py, x + width - 1, gradient(1.0, yratio));
            }
        }
    }

    /// Draws a straight line between the two points using the current stroke
    /// color
    pub fn stroke_line(&mut self, x: i64, y: i64, x2: i64, y2: i64) {
        if x == x2 {
            for py in y..y2 {
                self.stroke_point(x, py);
            }
            return;
        } else if y == y2 {
            for px in x..x2 {
                self.stroke_point(px, y);
            }
            return;
        }

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
            self.stroke_point(px, py);

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

    /// Draws a straight line between the two points using the current fill
    /// color
    pub fn fill_line(&mut self, x: i64, y: i64, x2: i64, y2: i64) {
        if x == x2 {
            for py in y..y2 {
                self.fill_point(x, py);
            }
            return;
        } else if y == y2 {
            for px in x..x2 {
                self.fill_point(px, y);
            }
            return;
        }

        let deltax = (x2 - x).abs();
        let stepx = (x2 - x).signum();

        let deltay = -(y2 - y).abs();
        let stepy = (y2 - y).signum();

        let mut error = deltax + deltay;

        let mut px = x;
        let mut py = y;
        loop {
            self.fill_point(px, py);

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

    /// Draws a straight line between the two points using the given gradient(ratio)
    pub fn gstroke_line<F>(&mut self, x: i64, y: i64, x2: i64, y2: i64, gradient: F)
    where
        F: Fn(f64) -> Element,
    {
        if x == x2 {
            let length = (y2 - y) as f64;
            for py in y..y2 {
                let yratio = (py - y) as f64 / length;
                self.buffer.put_point(x, py, gradient(yratio));
            }
            return;
        } else if y == y2 {
            let length = (x2 - x) as f64;
            for px in x..x2 {
                let xratio = (px - x) as f64 / length;
                self.buffer.put_point(px, y, gradient(xratio));
            }
            return;
        }

        let deltax = (x2 - x).abs();
        let stepx = (x2 - x).signum();

        let deltay = -(y2 - y).abs();
        let stepy = (y2 - y).signum();

        let mut error = deltax + deltay;
        let length = ((deltax as f64).powf(2.0) + (deltay as f64).powf(2.0)).sqrt();

        let mut px = x;
        let mut py = y;
        loop {
            let point_length = (((px - x) as f64).powf(2.0) + ((py - y) as f64).powf(2.0)).sqrt();

            self.buffer
                .put_point(px, py, gradient(point_length / length));

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

    /// Draws a circle's perimeter around the given point using the current
    /// stroke color
    pub fn stroke_circle(&mut self, x: i64, y: i64, r: i64) {
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
            self.stroke_point(x + relx, y + rely); // Quadrant II
            self.stroke_point(x - relx, y + rely); // Quadrant I
            self.stroke_point(x + relx, y - rely); // Quadrant IV
            self.stroke_point(x - relx, y - rely); // Quadrant III

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

    /// Draws a circle's perimeter around the given point using the given
    /// gradient(angle)
    pub fn gstroke_circle<F>(&mut self, x: i64, y: i64, r: i64, gradient: F)
    where
        F: Fn(f64) -> Element,
    {
        let mut error = -2 * r + 2;

        let mut relx = -r;
        let mut rely = 0;

        while relx <= 0 {
            let q1_angle = ((y + rely) as f64).atan2((x + relx) as f64);
            self.buffer
                .put_point(x + relx, y + rely, gradient(q1_angle));

            let q2_angle = ((y + rely) as f64).atan2((x - relx) as f64);
            self.buffer
                .put_point(x - relx, y + rely, gradient(q2_angle));

            let q3_angle = ((y - rely) as f64).atan2((x + relx) as f64);
            self.buffer
                .put_point(x + relx, y - rely, gradient(q3_angle));

            let q4_angle = ((y - rely) as f64).atan2((x - relx) as f64);
            self.buffer
                .put_point(x - relx, y - rely, gradient(q4_angle));

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

    /// Fills a circle around the given point
    pub fn fill_circle(&mut self, x: i64, y: i64, r: i64) {
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
                self.fill_line(x + relx, y + rely, x - relx, y + rely);
                self.fill_line(x + relx, y - rely, x - relx, y - rely);

                rely += 1;
                error += 2 * rely + 1;
            }
        }
    }

    /// Fills a circle around the given point using the given gradient(angle, radius)
    pub fn gfill_circle<F>(&mut self, x: i64, y: i64, r: i64, gradient: F)
    where
        F: Fn(f64, f64) -> Element,
    {
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
                let py = y - rely;
                let py2 = y + rely;
                for px in (x + relx)..(x - relx) {
                    let distance = (((px - x) as f64).powf(2.0) + ((py - y) as f64).powf(2.0))
                        .sqrt()
                        / (r as f64);

                    let angle = ((py - y) as f64).atan2((px - x) as f64);
                    let angle2 = ((py2 - y) as f64).atan2((px - x) as f64);
                    self.buffer.put_point(px, py, gradient(angle, distance));
                    self.buffer.put_point(px, py2, gradient(angle2, distance));
                }

                rely += 1;
                error += 2 * rely + 1;
            }
        }
    }
}
