use framebuffer::*;
use std::io;

fn main() {
    let background = Color::black();
    let red_50 = Color::rgba(255, 0, 0, 85);
    let blue_50 = Color::rgba(0, 0, 255, 85);
    let green_50 = Color::rgba(0, 255, 0, 85);

    let mut stdout = io::stdout();

    let mut gfx = Framebuffer::new(800, 800, background);
    gfx.fill_rect(0, 0, 500, 800, red_50);
    gfx.fill_rect(300, 0, 500, 800, blue_50);
    gfx.fill_rect(0, 0, 800, 500, green_50);
    gfx.write(&mut stdout).unwrap();
}
