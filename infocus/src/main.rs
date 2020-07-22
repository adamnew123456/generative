use framebuffer::*;
use random;
use std::time::{Duration, SystemTime};

const CANVAS_SIZE: i64 = 800;
const LENS_RADIUS: i64 = 50;

struct Lens {
    x: i64,
    y: i64,
    vx: i64,
    vy: i64,
    radius: i64,
}

impl Lens {
    fn new(x: i64, y: i64, vx: i64, vy: i64) -> Lens {
        Lens {
            x: x,
            y: y,
            vx: vx,
            vy: vy,
            radius: LENS_RADIUS,
        }
    }

    fn top(&self) -> i64 {
        self.y - self.radius
    }

    fn bottom(&self) -> i64 {
        self.y + self.radius
    }

    fn left(&self) -> i64 {
        self.x - self.radius
    }

    fn right(&self) -> i64 {
        self.x + self.radius
    }

    fn step(&mut self) {
        self.x += self.vx;
        self.y += self.vy;

        if self.top() <= 0 {
            self.y = LENS_RADIUS;
            self.vy *= -1;
        }

        if self.bottom() >= CANVAS_SIZE {
            self.y = CANVAS_SIZE - LENS_RADIUS;
            self.vy *= -1;
        }

        if self.left() <= 0 {
            self.x = LENS_RADIUS;
            self.vx *= -1;
        }

        if self.right() >= CANVAS_SIZE {
            self.x = CANVAS_SIZE - LENS_RADIUS;
            self.vx *= -1;
        }
    }

    fn contains(&self, x: i64, y: i64) -> bool {
        let distance = (x - self.x).pow(2) + (y - self.y).pow(2);
        distance <= self.radius.pow(2)
    }
}

/// Generates a seeded "random" value using the current process ID and time.
fn new_rng() -> random::Default {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .as_secs();
    random::default().seed([std::process::id() as u64, time])
}

/// Generates a random color
fn random_color<T: random::Source>(source: &mut T) -> Color {
    Color::rgb(
        source.read::<u8>(),
        source.read::<u8>(),
        source.read::<u8>(),
    )
}

fn main() {
    let mut rng = new_rng();

    let mut lenses = Vec::new();
    for x in 0..(CANVAS_SIZE / (LENS_RADIUS * 2)) {
        for y in 0..(CANVAS_SIZE / (LENS_RADIUS * 2)) {
            lenses.push(Lens::new(
                LENS_RADIUS + x * LENS_RADIUS * 2,
                LENS_RADIUS + y * LENS_RADIUS * 2,
                x + 1,
                y + 1,
            ));
        }
    }

    let framebuffer = FrameBuffer::new(CANVAS_SIZE as u32, CANVAS_SIZE as u32);
    let mut framegfx = Canvas::new(framebuffer, Color::white(), Color::black());

    let maskbuffer = StencilBuffer::new(CANVAS_SIZE as u32, CANVAS_SIZE as u32);
    let mut maskgfx = Canvas::new(maskbuffer, 0, 1);

    loop {
        maskgfx.fill();

        for x in 0..CANVAS_SIZE {
            for y in 0..CANVAS_SIZE {
                framegfx.put_point(x, y, random_color(&mut rng));
            }
        }

        for lens in lenses.iter_mut() {
            lens.step();
        }

        for lens in lenses.iter() {
            for x in lens.left()..lens.right() {
                for y in lens.top()..lens.bottom() {
                    if !lens.contains(x, y) {
                        continue;
                    }

                    maskgfx.put_point(x, y, maskgfx.get_point(x, y).unwrap() + 1);
                }
            }
        }

        framegfx.mask(&maskgfx, |color, mask| match mask {
            0 => Color::black(),
            1 => color.mask(true, false, false),
            2 => color.mask(false, true, false),
            3 => color.mask(false, false, true),
            _ => color,
        });

        framegfx.buffer().write(&mut std::io::stdout()).unwrap();
    }
}
