use framebuffer::*;
use random;
use random::Source;
use std::f64;
use std::io;
use std::time::{Duration, SystemTime};

/// Generates a seeded "random" value using the current process ID and time.
fn new_rng() -> random::Default {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .as_secs();
    random::default().seed([std::process::id() as u64, time])
}

const CENTER_X: i64 = 400;
const CENTER_Y: i64 = 400;

const CORE_MAX_SIZE: i64 = 250;
const CORE_MIN_ENERGY: i64 = 0;
const CORE_MAX_ENERGY: i64 = 175;
const CORE_BLEED_RATE: i64 = 4;
const CORE_CHARGE_RATE: i64 = 10;
const HALO_MAX_SIZE: i64 = 30;
const HALO_MIN_SIZE: i64 = 10;
const HALO_JITTER: i64 = 5;

const ACCUMULATOR_COUNT: usize = 10;
const ACCUMULATOR_RADIUS: f64 = 300.0;
const ACCUMULATOR_SIZE: i64 = 35;
const ACCUMULATOR_RATE: f64 = 200.0;
const ACCUMULATOR_HEAT: u8 = 25;
const ACCUMULATOR_COOL: u8 = 1;
const ACCUMULATOR_MIN_HEAT: u8 = 50;

fn main() {
    let mut stdout = io::stdout();
    let background = Color::black();
    let blur = Color::rgba(0, 0, 0, 15);
    let bolt = Color::white();
    let fill = Color::rgba(255, 255, 0, 120);
    let fill_halo = Color::rgba(255, 0, 0, 200);
    let buffer = FrameBuffer::new((CENTER_X * 2) as u32, (CENTER_Y * 2) as u32);
    let mut gfx = Canvas::new(buffer, background, Color::white());
    let mut rng = new_rng();

    // Main core state - energy (determines radius) and bleeding (determines
    // bolts)
    let mut energy = 0;
    let mut bleeding = false;

    // Used for core halo, randomly adjusted by a few pixels each step to
    // maintain
    let mut jitter = 0;
    let mut target = 0;

    let accumulator_step = (2.0 * f64::consts::PI) / ACCUMULATOR_RATE;
    let mut accumulator_offset = 0.0;
    let mut accumulator_heat: [u8; ACCUMULATOR_COUNT] = [0; ACCUMULATOR_COUNT];
    let mut accumulator_base_angles: [f64; ACCUMULATOR_COUNT] = [0.0; ACCUMULATOR_COUNT];
    for i in 0..ACCUMULATOR_COUNT {
        accumulator_base_angles[i] = (i as f64) * (2.0 * f64::consts::PI) / (ACCUMULATOR_COUNT as f64);
    }

    loop {
        gfx.set_fill(blur);
        gfx.fill();

        if bleeding {
            energy -= CORE_BLEED_RATE;
            energy = energy.max(CORE_MIN_ENERGY);
        } else {
            energy += rng.read::<i64>().abs() % CORE_CHARGE_RATE;
            energy = energy.min(CORE_MAX_ENERGY);
        }

        accumulator_offset += accumulator_step;
        if accumulator_offset > 2.0 * f64::consts::PI {
            accumulator_offset -= 2.0 * f64::consts::PI;
        }

        jitter += rng.read::<i64>() % HALO_JITTER;
        jitter = jitter.max(HALO_MIN_SIZE).min(HALO_MAX_SIZE);
        let radius = CORE_MAX_SIZE - energy;

        for i in 0..ACCUMULATOR_COUNT {
            let angle = accumulator_base_angles[i] + accumulator_offset;
            let x = (ACCUMULATOR_RADIUS * angle.cos()) as i64 + CENTER_X;
            let y = (ACCUMULATOR_RADIUS * angle.sin()) as i64 + CENTER_Y;

            if bleeding && i == target {
                let offset = ACCUMULATOR_SIZE / 2;
                gfx.set_stroke(bolt);
                gfx.stroke_line(CENTER_X, CENTER_Y, x, y);
                gfx.stroke_line(CENTER_X, CENTER_Y, x - offset, y - offset);
                gfx.stroke_line(CENTER_X, CENTER_Y, x + offset, y - offset);
                gfx.stroke_line(CENTER_X, CENTER_Y, x - offset, y + offset);
                gfx.stroke_line(CENTER_X, CENTER_Y, x + offset, y + offset);

                if 255 - accumulator_heat[i] >= ACCUMULATOR_HEAT {
                    accumulator_heat[i] += ACCUMULATOR_HEAT;
                } else {
                    accumulator_heat[i] = 255;
                }
            } else if accumulator_heat[i] >= ACCUMULATOR_COOL {
                accumulator_heat[i] -= ACCUMULATOR_COOL;
            } else {
                accumulator_heat[i] = 0;
            }

            let color = Color::rgb(0, accumulator_heat[i], accumulator_heat[i]);
            gfx.set_fill(color);
            gfx.fill_circle(x, y, ACCUMULATOR_SIZE);
        }

        // Draw the core
        gfx.set_fill(fill);
        gfx.gfill_circle(CENTER_X, CENTER_Y, radius + jitter, |_, radius| {
            let mix = Color::rgba(0, 0, 0, 200 - (50.0 * radius) as u8);
            fill.blend(mix)
        });

        // Draw the top-reaching bolts over the core
        for i in 0..ACCUMULATOR_COUNT {
            if (bleeding && target == i) || accumulator_heat[i] < ACCUMULATOR_MIN_HEAT {
                continue;
            }

            let angle = accumulator_base_angles[i] + accumulator_offset;
            let x = (ACCUMULATOR_RADIUS * angle.cos()) as i64 + CENTER_X;
            let y = (ACCUMULATOR_RADIUS * angle.sin()) as i64 + CENTER_Y;

            let color = Color::rgb(0, accumulator_heat[i], accumulator_heat[i]);
            let offset = ACCUMULATOR_SIZE / 2;
            gfx.set_stroke(color);
            gfx.stroke_line(x, y, CENTER_X, -100);
            gfx.stroke_line(x - offset, y - offset, CENTER_X, -100);
            gfx.stroke_line(x + offset, y - offset, CENTER_X, -100);
            gfx.stroke_line(x - offset, y + offset, CENTER_X, -100);
            gfx.stroke_line(x + offset, y + offset, CENTER_X, -100);
        }

        // Post-fill the halo so it affects all the energy bolts, including a
        // shading gradient that gets darker toward the center of the core
        gfx.gfill_circle(CENTER_X, CENTER_Y, radius, |_, radius| {
            let mix = Color::rgba(0, 0, 0, 200 - (100.0 * radius.min(0.50)) as u8);
            fill_halo.blend(mix)
        });

        gfx.buffer().write(&mut stdout).unwrap();

        if bleeding && energy == CORE_MIN_ENERGY {
            bleeding = false;
            target = (target + 1) % ACCUMULATOR_COUNT;
        } else if energy == CORE_MAX_ENERGY {
            bleeding = true;
        }
    }
}
