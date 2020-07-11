use std::io;
use std::io::prelude::*;
use framebuffer::*;

const CELL_SIZE: i64 = 20;
const CELL_GAP: i64 = 4;

const HEATING: u8 = 120;
const COOLING: u8 = 1;

fn cool_heatmap(heatmap: &mut [u8; 256]) {
    for i in 0..255 {
        if heatmap[i] > COOLING {
            heatmap[i] -= COOLING;
        } else {
            heatmap[i] = 0;
        }
    }
}

fn render_frame<T: io::Write>(gfx: &mut Framebuffer, heatmap: &[u8; 256], stream: &mut T) -> io::Result<()> {
    let mut x = CELL_GAP;
    let mut y = CELL_GAP;

    for i in 0..256 {
        let byteval = heatmap[i];
        let color = Color::new(byteval, 0, 0);
        gfx.fill_rect(x, y, CELL_SIZE, CELL_SIZE, color);

        if (i + 1) % 16 == 0 {
            x = CELL_GAP;
            y += CELL_SIZE + CELL_GAP;
        } else {
            x += CELL_SIZE + CELL_GAP;
        }
    }

    gfx.write(stream)
}

fn main() {
    let background = Color::white();
    let dimension = (CELL_GAP + CELL_SIZE) * 16 + CELL_GAP;
    let mut stdout = io::stdout();
    let mut stdin = io::stdin();
    let mut gfx = Framebuffer::new(dimension as u32, dimension as u32, background);

    let mut heatmap: [u8; 256] = [0; 256];
    render_frame(&mut gfx, &heatmap, &mut stdout).unwrap();
    gfx.fill(background);

    let mut byteval: [u8; 1] = [0; 1];
    loop {
        let size = stdin.read(&mut byteval).unwrap();
        if size == 0 {
            break
        }

        if heatmap[byteval[0] as usize] < 255 - HEATING {
            heatmap[byteval[0] as usize] += HEATING;
        }

        render_frame(&mut gfx, &heatmap, &mut stdout).unwrap();
        gfx.fill(background);

        cool_heatmap(&mut heatmap);
    }
}
