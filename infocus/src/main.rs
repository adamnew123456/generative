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
fn random_color<T: random::Source>(source: &mut T, r: bool, g: bool, b: bool) -> Color {
    Color::new(if r { source.read::<u8>() } else { 0 },
               if g { source.read::<u8>() } else { 0 },
               if b { source.read::<u8>() } else { 0 })
}

fn main() {
    let mut rng = new_rng();

    let mut lenses = Vec::new();
    for x in 0..(CANVAS_SIZE / (LENS_RADIUS * 2)) {
        for y in 0..(CANVAS_SIZE / (LENS_RADIUS * 2)) {
            lenses.push(Lens::new(LENS_RADIUS + x * LENS_RADIUS * 2,
                                  LENS_RADIUS + y * LENS_RADIUS * 2,
                                  x + 1,
                                  y + 1));
        }
    }

    let mut framebuffer = Framebuffer::new(CANVAS_SIZE as u32, CANVAS_SIZE as u32, Color::white());
    let black = Color::black();

    for _ in 0..630 {
        for lens in lenses.iter_mut() {
            lens.step();
        }

        for x in 0..CANVAS_SIZE {
            for y in 0..CANVAS_SIZE {
                let mut focus = 0;
                for lens in lenses.iter() {
                    if lens.contains(x, y) {
                        focus += 1;
                    }
                }

                let pixel =
                    if focus == 0 {
                        black
                    } else {
                        random_color(&mut rng,
                                     focus == 1 || focus > 3,
                                     focus == 2 || focus > 3,
                                     focus >= 3)
                    };
                framebuffer.point_at(x, y, pixel);
            }
        }

        framebuffer.write(&mut std::io::stdout()).unwrap();
    }
}
