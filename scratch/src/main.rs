use framebuffer::*;
use std::io;

fn main() {
    let background = Color::white();
    let red = Color::rgb(255, 0, 0);
    let green = Color::rgb(0, 255, 0);
    let blue = Color::rgb(0, 0, 255);

    let mut stdout = io::stdout();

    let mut gfx = Framebuffer::new(400, 400, background);

    let (mut a, mut b, mut c) = (red, green, blue);
    for _i in 0..(15 * 30) {
        for r in 1..80 {
            gfx.circle_at(200, 200, r * 3, a);
            gfx.circle_at(200, 200, (r * 3) + 1, b);
            gfx.circle_at(200, 200, (r * 3) + 2, c);
        }

        gfx.write(&mut stdout).unwrap();

        let tmp = a;
        a = b;
        b = c;
        c = tmp;
    }
}
