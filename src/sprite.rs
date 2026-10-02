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
    pub bank_angle: f32,
    pub banking: i32,
    pub pitching: i32,
    pub angle_attack: f32,
    pub landed: bool,
    pub flag: bool,
    pub tracking: bool,
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
        tracking: false,
        landed: false,
        flag: false,
        banking: 0,
        pitching: 0,
        width: img.width(),
        pos,
        //x,
        //y,
        //z: 550.,
        bank_angle: 0.,
        angle_attack: 0.,
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
pub fn draw_sprite3(
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

    let hs = 2.;
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
pub fn draw_sprite(
    frame: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    origin_x: f32,
    origin_y: f32,
    zoom: f32,
    tile_width: u32,
    pos: Vec3,
    sprite: &Sprite,
) {
    let footprint_w = tile_width as f32 * zoom;
    let footprint_h = footprint_w / 2.0;
    let hs = 2.;

    let iso_x = origin_x + (pos.x - pos.y) * (footprint_w / 2.0);
    let iso_y = origin_y + (pos.x + pos.y) * (footprint_h / 2.0);

    // Shadow: once per sprite, and it does NOT rotate with the bank angle
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

    let src_w = sprite.width as usize;
    let src_h = sprite.height as usize;
    let scaled_w = sprite.width as f32 * zoom;
    let scaled_h = sprite.height as f32 * zoom;

    // bank_angle is in degrees
    let (sin_a, cos_a) = sprite.bank_angle.to_radians().sin_cos();

    // Rotate around the sprite's center, which stays where the unrotated sprite's center was
    let cx = iso_x + scaled_w / 2.0;
    let cy = iso_y - (pos.z * hs) * zoom + scaled_h / 2.0;

    // Bounding box of the rotated rectangle
    let half_bw = (scaled_w * cos_a.abs() + scaled_h * sin_a.abs()) / 2.0;
    let half_bh = (scaled_w * sin_a.abs() + scaled_h * cos_a.abs()) / 2.0;

    // Clip the loop to the visible screen
    let x0 = ((cx - half_bw).floor() as isize).max(0);
    let x1 = ((cx + half_bw).ceil() as isize).min(screen_width as isize);
    let y0 = ((cy - half_bh).floor() as isize).max(0);
    let y1 = ((cy + half_bh).ceil() as isize).min(screen_height as isize);

    for sy in y0..y1 {
        for sx in x0..x1 {
            // Offset from the center, then rotate by -angle to find the source position
            let dx = sx as f32 + 0.5 - cx;
            let dy = sy as f32 + 0.5 - cy;
            let rx = dx * cos_a + dy * sin_a;
            let ry = -dx * sin_a + dy * cos_a;

            // Back to sprite pixel coordinates (undo zoom, shift origin to top-left)
            let src_x = (rx + scaled_w / 2.0) / zoom;
            let src_y = (ry + scaled_h / 2.0) / zoom;

            if src_x < 0.0 || src_y < 0.0 || src_x >= src_w as f32 || src_y >= src_h as f32 {
                continue;
            }

            let px = sprite.pxs[src_y as usize * src_w + src_x as usize];
            if px.3 == 0 {
                continue;
            }

            let offset = (sy as usize * screen_width + sx as usize) * 4;
            frame[offset..offset + 4].copy_from_slice(&[px.0, px.1, px.2, px.3]);
        }
    }
}
