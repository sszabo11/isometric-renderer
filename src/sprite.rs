use anyhow::Result;
use image::{GenericImageView, ImageReader};

use crate::Px;

pub struct Sprite {
    pub pxs: Vec<Px>,
    pub width: u32,
    pub height: u32,
}
pub fn load_sprite(path: &str) -> Result<Sprite> {
    let img = ImageReader::open(path)?.decode()?;

    let pxs = img
        .pixels()
        .map(|(_x, _y, color)| (color[0], color[1], color[2], color[3]))
        .collect();

    Ok(Sprite {
        pxs,
        width: img.width(),
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
    grid_x: f32,     // entity's grid-space position (can be fractional for smooth movement)
    grid_y: f32,
    sprite: &Sprite, // pre-loaded, not a path
) {
    let footprint_w = tile_width as f32 * zoom;
    let footprint_h = footprint_w / 2.0;

    let iso_x = origin_x + (grid_x - grid_y) * (footprint_w / 2.0);
    let iso_y = origin_y + (grid_x + grid_y) * (footprint_h / 2.0);

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
            let screen_y = iso_y as isize + py as isize;

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
