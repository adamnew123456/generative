use framebuffer::*;
use std::io;

fn main() {
    let red = Color::rgb(255, 0, 0);
    let green = Color::rgb(0, 255, 0);
    let blue = Color::rgb(0, 0, 255);

    let mut stdout = io::stdout();

    let buffer = FrameBuffer::new(400, 400);
    let mut gfx = Canvas::new(buffer, Color::white(), Color::black());

    let (mut a, mut b, mut c) = (red, green, blue);
    for _i in 0..(15 * 30) {
        for r in 1..80 {
            gfx.set_stroke(a);
            gfx.stroke_circle(200, 200, r * 3);
            gfx.set_stroke(b);
            gfx.stroke_circle(200, 200, (r * 3) + 1);
            gfx.set_stroke(c);
            gfx.stroke_circle(200, 200, (r * 3) + 2);
        }

        gfx.buffer().write(&mut stdout).unwrap();

        let tmp = a;
        a = b;
        b = c;
        c = tmp;
    }
}
