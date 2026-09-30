use anyhow::Result;
use image::{GenericImageView, ImageReader};

use crate::{grid::Px, vec::Vec3};

pub struct Sprite {
    pub pxs: Vec<Px>,
    pub width: u32,
    pub height: u32,
    pub acc: Vec3,
    pub vel: Vec3,
    pub pos: Vec3,
    pub mass: f32,
    pub flag: bool,
}
pub fn load_sprite(path: &str, pos: Vec3) -> Result<Sprite> {
    let img = ImageReader::open(path)?.decode()?;

    let pxs = img
        .pixels()
        .map(|(_x, _y, color)| (color[0], color[1], color[2], color[3]))
        .collect();

    Ok(Sprite {
        pxs,
        mass: 1.,
        flag: false,
        width: img.width(),
        pos,
        //x,
        //y,
        //z: 550.,
        vel: Vec3::default(),
        acc: Vec3::default(),
        height: img.height(),
    })
}
pub fn draw_sprite2(
    frame: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    origin_x: f32,
    origin_y: f32,
    zoom: f32,
    x: u32,
    y: u32,
    sprite: &Sprite,
) {
    let x_start = origin_x as isize;
    let y_start = origin_y as isize;

    let iso_x = x_start + (x as isize - y as isize);
    let iso_y = y_start + (x as isize + y as isize);
    for py in 0..sprite.height {
        for px in 0..sprite.width {
            let src_x = ((px as f32 / zoom) as usize).min(screen_width - 1);
            let src_y = ((py as f32 / zoom) as usize).min(screen_height - 1);

            let sprite_px = sprite.pxs[src_y * screen_width as usize + src_x];
            let screen_x = iso_x + px as isize;
            //let screen_y = iso_y + py as isize;
            let screen_y = iso_y + py as isize;

            if screen_x < 0
                || screen_y < 0
                || screen_x as usize >= screen_width
                || screen_y as usize >= screen_height
            {
                continue;
            }

            let pixel_idx = screen_y as usize * screen_width + screen_x as usize;
            let offset = pixel_idx * 4;

            //let tile_px = tile.pxs[py * tile_width + px];

            if sprite_px.3 == 0 {
                continue;
            }
            frame[offset..offset + 4].copy_from_slice(&[
                sprite_px.0,
                sprite_px.1,
                sprite_px.2,
                sprite_px.3,
            ]);
        }
    }
}
pub fn draw_sprite(
    frame: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    origin_x: f32,
    origin_y: f32,
    zoom: f32,
    tile_width: u32, // needed to compute footprint spacing, matching tile draw()
    pos: Vec3,
    sprite: &Sprite, // pre-loaded, not a path
) {
    let footprint_w = tile_width as f32 * zoom;
    let footprint_h = footprint_w / 2.0;

    let hs = 20.;
    let iso_x = origin_x + (pos.x - pos.y) * (footprint_w / 2.0);
    let iso_y = origin_y + (pos.x + pos.y) * (footprint_h / 2.0);

    let scaled_w = (sprite.width as f32 * zoom) as usize;
    let scaled_h = (sprite.height as f32 * zoom) as usize;

    for py in 0..scaled_h {
        for px in 0..scaled_w {
            let src_x = ((px as f32 / zoom) as usize).min(sprite.width as usize - 1);
            let src_y = ((py as f32 / zoom) as usize).min(sprite.height as usize - 1);

            let sprite_px = sprite.pxs[src_y * sprite.width as usize + src_x];
            if sprite_px.3 == 0 {
                continue;
            }

            let screen_x = iso_x as isize + px as isize;
            let screen_y = iso_y as isize + py as isize - ((pos.z * hs) * zoom) as isize;

            // shadow
            let (shadow_x, shadow_y) = project(
                origin_x,
                origin_y,
                zoom,
                tile_width as f32,
                pos.x,
                pos.y,
                0.,
            );

            draw_shadow(
                frame,
                screen_width,
                screen_height,
                shadow_x as isize + 20,
                shadow_y as isize + 10,
                15.0,
                6.0,
                0.35,
            );
            //{
            //    let shadow_x = screen_x - 10;
            //    let shadow_y = screen_y - 10;
            //    let a = 30;
            //    let b = 4;

            //    let a_h = a / 2;
            //    let b_h = b / 2;

            //    let mut y = a_h;
            //    let mut x = 0;

            //    while y >= -a_h {
            //        let i1 = ((y + shadow_y) * screen_width as isize + x + shadow_x) as usize;
            //        let i2 = ((y + shadow_y) * screen_width as isize - x + shadow_x) as usize;
            //        let offset1 = i1 * 4;
            //        let offset2 = i2 * 4;

            //        frame[offset1..offset1 + 4].copy_from_slice(&[255, 255, 255, 255]);
            //        //frame[i2..i2 + 4].copy_from_slice(&[255, 255, 0, 255]);
            //        y -= 1;
            //    }
            //}
            if screen_x < 0
                || screen_y < 0
                || screen_x as usize >= screen_width
                || screen_y as usize >= screen_height
            {
                continue;
            }

            let pixel_idx = screen_y as usize * screen_width + screen_x as usize;
            let offset = pixel_idx * 4;
            frame[offset..offset + 4].copy_from_slice(&[
                sprite_px.0,
                sprite_px.1,
                sprite_px.2,
                sprite_px.3,
            ]);
        }
    }
}

fn draw_shadow(
    frame: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    center_x: isize,
    center_y: isize,
    radius_x: f32,
    radius_y: f32,
    alpha: f32, // 0.0 to 1.0, how dark the shadow is
) {
    let ry = radius_y.ceil() as isize;

    for dy in -ry..=ry {
        let t = dy as f32 / radius_y;
        if t * t > 1.0 {
            continue; // outside the ellipse vertically
        }
        let half_width = radius_x * (1.0 - t * t).sqrt();
        let rx = half_width.round() as isize;

        let py = center_y + dy;
        if py < 0 || py as usize >= screen_height {
            continue;
        }

        let x_start = (center_x - rx).max(0);
        let x_end = (center_x + rx).min(screen_width as isize - 1);

        for px in x_start..=x_end {
            let idx = (py as usize * screen_width + px as usize) * 4; // byte offset, *4 is essential
            let bg = [frame[idx], frame[idx + 1], frame[idx + 2]];

            // blend toward black by `alpha` — this is what makes it read as a soft shadow
            let blended = [
                (bg[0] as f32 * (1.0 - alpha)) as u8,
                (bg[1] as f32 * (1.0 - alpha)) as u8,
                (bg[2] as f32 * (1.0 - alpha)) as u8,
            ];

            frame[idx..idx + 3].copy_from_slice(&blended);
            // alpha channel (idx+3) untouched — leave the background's existing alpha as-is
        }
    }
}

fn project(
    origin_x: f32,
    origin_y: f32,
    zoom: f32,
    tile_width: f32,
    grid_x: f32,
    grid_y: f32,
    z: f32,
) -> (f32, f32) {
    let footprint_w = tile_width * zoom;
    let footprint_h = footprint_w / 2.0; // enforced 2:1 ratio

    let screen_x = origin_x + (grid_x - grid_y) * (footprint_w / 2.0);
    let screen_y = origin_y + (grid_x + grid_y) * (footprint_h / 2.0) - z * zoom;

    (screen_x, screen_y)
}
