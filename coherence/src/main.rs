#[macro_use]
extern crate bitflags;

use framebuffer::*;
use random;
use std::io;
use std::time::{Duration, SystemTime};

const CANVAS_SIZE: i64 = 400;
const GRID_SIZE: i64 = 4;
const CELL_COUNT: i64 = CANVAS_SIZE / GRID_SIZE;

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

bitflags! {
    struct GridSides: u32 {
        const NONE = 0;
        const LEFT = 1;
        const RIGHT = 2;
        const TOP = 4;
        const BOTTOM = 8;
    }
}

struct Grid {
    cells: Vec<Color>,
    infected: Vec<u8>,
}

impl Grid {
    fn new<T: random::Source>(rng: &mut T) -> Grid {
        let mut cells = Vec::with_capacity((CELL_COUNT * CELL_COUNT) as usize);
        let mut infected = Vec::with_capacity((CELL_COUNT * CELL_COUNT) as usize);
        for _ in 0..(CELL_COUNT * CELL_COUNT) {
            let color = random_color(rng);
            cells.push(color);
            if color.r == 0{
                infected.push(1);
            } else if color.g == 0 {
                infected.push(2);
            } else if color.b == 0 {
                infected.push(3);
            } else {
                infected.push(0);
            }
        }

        infected[Grid::offset(0, 0).unwrap()] = 4;
        cells[Grid::offset(0, 0).unwrap()] = Color::black();

        infected[Grid::offset(CELL_COUNT - 1, 0).unwrap()] = 4;
        cells[Grid::offset(CELL_COUNT - 1, 0).unwrap()] = Color::black();

        infected[Grid::offset(0, CELL_COUNT - 1).unwrap()] = 4;
        cells[Grid::offset(0, CELL_COUNT - 1).unwrap()] = Color::black();

        infected[Grid::offset(CELL_COUNT - 1, CELL_COUNT - 1).unwrap()] = 4;
        cells[Grid::offset(CELL_COUNT - 1, CELL_COUNT - 1).unwrap()] = Color::black();

        Grid {
            cells,
            infected,
        }
    }

    fn offset(x: i64, y: i64) -> Option<usize> {
        if x < 0 || x >= CELL_COUNT || y < 0 || y >= CELL_COUNT {
            None
        } else {
            Some((y * CELL_COUNT + x) as usize)
        }
    }

    fn get_at(&self, x: i64, y: i64) -> Color {
        Grid::offset(x, y)
            .map(|offset| self.cells[offset as usize])
            .unwrap_or(Color::black())
    }

    fn get_borders(&self, x: i64, y: i64) -> GridSides {
        let cell = self.get_at(x, y);
        let top = self.get_at(x, y - 1);
        let bottom = self.get_at(x, y + 1);
        let left = self.get_at(x - 1, y);
        let right = self.get_at(x + 1, y);

        let mut mask = GridSides::NONE;
        if cell != top {
            mask = mask | GridSides::TOP;
        }

        if cell != bottom {
            mask = mask | GridSides::BOTTOM;
        }

        if cell != left {
            mask = mask | GridSides::LEFT;
        }

        if cell != right {
            mask = mask | GridSides::RIGHT;
        }

        mask
    }

    fn update_infection(&mut self, x: i64, y: i64) {
        let offset = Grid::offset(x, y).unwrap();

        let infected = self.infected[offset as usize];
        let color = self.cells[offset as usize];
        if infected == 0 {
            return;
        }

        match infected {
            1 => {
                if color.r < 255 {
                    let new_color = Color::rgb(color.r + 1, 0, 0);
                    self.cells[offset as usize] = new_color;
                    return
                }
            }
            2 => {
                if color.g < 255 {
                    let new_color = Color::rgb(0, color.g + 1, 0);
                    self.cells[offset as usize] = new_color;
                    return;
                }
            }
            3 => {
                if color.b < 255 {
                    let new_color = Color::rgb(0, 0, color.b + 1);
                    self.cells[offset as usize] = new_color;
                    return;
                }
            }
            4 => {
                if color.r < 255 {
                    let new_color = Color::rgb(color.r + 1, color.g + 1, color.b + 1);
                    self.cells[offset as usize] = new_color;
                    return;
                }
            }
            _ => return,
        };

        let mut update = |x, y| {
            Grid::offset(x, y)
                .map(|offset| {
                    // Only spread white to previously infected cells
                    if self.infected[offset] > 0 && self.infected[offset] < 4 && infected == 4 {
                        self.infected[offset] = 4;
                        self.cells[offset] = Color::rgb(252, 252, 252);
                    } else if self.infected[offset] == 0 && infected != 4 {
                        self.infected[offset] = infected;
                    }
                });
        };

        update(x - 1, y);
        update(x + 1, y);
        update(x, y - 1);
        update(x, y + 1);
    }

    fn step(&mut self) {
        for y in 0..CELL_COUNT {
            for x in 0..CELL_COUNT {
                self.update_infection(x, y);
            }
        }
    }

    fn draw<Buffer: GraphicBuffer<Color>>(&self, canvas: &mut Canvas<Color, Buffer>) {
        canvas.set_fill(Color::white());
        canvas.fill();

        for y in 0..CELL_COUNT {
            for x in 0..CELL_COUNT {
                let leftx = x * GRID_SIZE;
                let rightx = leftx + GRID_SIZE;
                let topy = y * GRID_SIZE;
                let bottomy = topy + GRID_SIZE;

                canvas.set_fill(self.get_at(x, y));
                canvas.fill_rect(leftx, topy, GRID_SIZE, GRID_SIZE);

                canvas.set_stroke(Color::black());
                let border = self.get_borders(x, y);

                if border & GridSides::TOP == GridSides::TOP {
                    canvas.stroke_line(leftx, topy, rightx, topy);
                }

                if border & GridSides::BOTTOM == GridSides::BOTTOM {
                    canvas.stroke_line(leftx, bottomy, rightx, bottomy);
                }

                if border & GridSides::LEFT == GridSides::LEFT {
                    canvas.stroke_line(leftx, topy, leftx, bottomy);
                }

                if border & GridSides::RIGHT == GridSides::RIGHT {
                    canvas.stroke_line(rightx, topy, rightx, bottomy);
                }
            }
        }
    }
}

fn main() {
    let mut stdout = io::stdout();
    let buffer = FrameBuffer::new(CANVAS_SIZE as u32, CANVAS_SIZE as u32);
    let mut gfx = Canvas::new(buffer, Color::white(), Color::black());

    let mut rng = new_rng();
    let mut grid = Grid::new(&mut rng);

    for _ in 0..2000 {
        gfx.fill();
        grid.step();
        grid.draw(&mut gfx);

        gfx.buffer().write(&mut stdout).unwrap();
    }
}
