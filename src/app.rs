use std::{fs, sync::Arc};

use anyhow::Result;
use fontdue::Font;
use image::{GenericImageView, ImageReader};
use pixels::{Error, Pixels, SurfaceTexture};
use rand::{RngExt, SeedableRng, rand_core::block::Generator};
use winit::{
    application::ApplicationHandler,
    event::{Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use crate::{
    Grid, Px, Tile,
    sprite::{Sprite, draw_sprite, load_sprite},
};

pub struct App {
    pub grid: Grid,
    pub window: Option<Arc<Window>>,
    pub pixels: Option<Pixels<'static>>,
    pub screen_width: u32,
    pub screen_height: u32,
    pub grid_tiles_width: u32,
    pub grid_tiles_height: u32,
    pub font: fontdue::Font,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub sprites: Vec<Sprite>,
    pub tile_n: u32,
    pub zoom: f32,
}

pub fn read_tile_file(path: &str) -> (Vec<Tile>, u32, u32, u32, u32) {
    let content = fs::read_to_string(path).expect("Failed to read tiles file");

    let lines = content.lines().collect::<Vec<&str>>();
    let grid_w = lines[0].parse::<u32>().unwrap();
    let grid_h = lines[1].parse::<u32>().unwrap();
    let tile_width = lines[2].parse::<u32>().unwrap();
    let tile_height = lines[3].parse::<u32>().unwrap();
    let mut tiles = Vec::new();
    for i in 4..lines.len() {
        let tile = lines[i];

        let mut t = Tile::empty(tile_width as usize, tile_height as usize);
        t.pxs.clear();
        for px_line in tile.split("[") {
            let values: Vec<&str> = px_line.split_whitespace().collect();
            println!("{:?}", values);
            let d: Vec<String> = values
                .iter()
                .map(|val| {
                    let n_str = val
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>();
                    n_str
                })
                .filter(|c| !c.is_empty())
                .collect();

            let px: Vec<u8> = d
                .iter()
                .map(|str_v| {
                    let n = str_v.parse::<u8>().expect("Invalid number");
                    n
                })
                .collect();
            if px.len() == 4 {
                t.pxs.push((px[0], px[1], px[2], px[3]));
            }
        }
        tiles.push(t);
    }
    (tiles, tile_width, tile_height, grid_w, grid_h)
}

//impl Default for App {
//    fn default() -> Self {
//        Self {
//            grid: Grid::empty(32, 32, 32, 32),
//            pixels: None,
//            window: None,
//            grid_tiles_width: 32,
//            grid_tiles_height: 32,
//            screen_width: 32 * 32,
//            screen_height: 32 * 32,
//            font: None,
//            mouse_x: 0.,
//            mouse_y: 0.,
//            origin_y: 0,
//            tile_n: 1,
//            origin_x: 0,
//        }
//    }
//}

fn draw_text(
    frame: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    font: &fontdue::Font,
    text: &str,
    start_x: i32,
    start_y: i32,
    size: f32,
    color: [u8; 3], // r, g, b
) {
    let mut pen_x = start_x;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);

        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let coverage = bitmap[gy * metrics.width + gx]; // 0..=255 alpha
                if coverage == 0 {
                    continue;
                }

                let screen_x = pen_x + metrics.xmin + gx as i32;
                let screen_y = start_y - metrics.ymin - (metrics.height as i32) + gy as i32;

                if screen_x < 0
                    || screen_y < 0
                    || screen_x as usize >= screen_width
                    || screen_y as usize >= screen_height
                {
                    continue;
                }

                let pixel_idx = screen_y as usize * screen_width + screen_x as usize;
                let offset = pixel_idx * 4;

                // simple alpha blend against whatever's already there
                let bg = &frame[offset..offset + 3];
                let blended = [
                    ((color[0] as u32 * coverage as u32 + bg[0] as u32 * (255 - coverage) as u32)
                        / 255) as u8,
                    ((color[1] as u32 * coverage as u32 + bg[1] as u32 * (255 - coverage) as u32)
                        / 255) as u8,
                    ((color[2] as u32 * coverage as u32 + bg[2] as u32 * (255 - coverage) as u32)
                        / 255) as u8,
                ];

                frame[offset..offset + 4]
                    .copy_from_slice(&[blended[0], blended[1], blended[2], 255]);
            }
        }

        pen_x += metrics.advance_width as i32;
    }
}
impl App {
    pub fn draw(&mut self) -> Result<(), Error> {
        let base_canvas_w = self.grid.tile_width as usize;
        let base_canvas_h = self.grid.tile_height as usize;
        let tile_width = (self.grid.tile_width as f32 * self.zoom) as usize;
        let base_footprint_w = base_canvas_w;

        let canvas_h = (base_canvas_h as f32 * self.zoom) as usize;
        let footprint_w = (base_footprint_w as f32 * self.zoom) as usize;
        let footprint_h = footprint_w / 2;

        let tile_height = (self.grid.tile_height as f32 * self.zoom) as usize;
        let screen_width = self.screen_width as usize;
        let screen_height = self.screen_height as usize;

        let x_start = self.origin_x as isize;
        let y_start = self.origin_y as isize;

        let (selected_x, selected_y) = self.get_tile(self.mouse_x, self.mouse_y);
        if let Some(pixels) = self.pixels.as_mut() {
            let frame = pixels.frame_mut();
            frame.fill(0);

            // Get tile place
            for ty in 0..self.grid.height as usize {
                for tx in 0..self.grid.width as usize {
                    let tile_idx = ty * self.grid.width as usize + tx;
                    let tile = &self.grid.tiles[tile_idx];

                    let iso_x = x_start + (tx as isize - ty as isize) * (footprint_w as isize / 2);
                    let iso_y = y_start + (tx as isize + ty as isize) * (footprint_h as isize / 2)
                        - (canvas_h as isize - footprint_h as isize);

                    // Fill inside tile
                    for py in 0..tile_height {
                        for px in 0..tile_width {
                            let src_x = ((px as f32 / self.zoom) as usize).min(base_canvas_w - 1);
                            let src_y = ((py as f32 / self.zoom) as usize).min(base_canvas_h - 1);

                            let tile_px = tile.pxs[src_y * self.grid.tile_width as usize + src_x];
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

                            if tile_px.3 == 0 {
                                continue;
                            }
                            frame[offset..offset + 4]
                                .copy_from_slice(&[tile_px.0, tile_px.1, tile_px.2, tile_px.3]);
                        }
                    }
                }
            }
            draw_sprite(
                frame,
                screen_width,
                screen_height,
                self.origin_x,
                self.origin_y,
                self.zoom,
                self.grid.tile_width,
                3.,
                15.,
                &self.sprites[0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Tile number: {}", self.tile_n),
                100,
                60,
                36.0,
                [255, 0, 0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Mouse: ({:.2}, {:.2})", self.mouse_x, self.mouse_y),
                100,
                100,
                36.0,
                [255, 0, 0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Tile: ({:.2}, {:.2})", selected_x, selected_y),
                100,
                140,
                36.0,
                [255, 0, 0],
            );
            if let Err(err) = pixels.render() {
                eprintln!("pixels.render error: {err}");
                return Err(err);
            }
        }

        Ok(())
    }

    fn get_tile(&self, mouse_x: f32, mouse_y: f32) -> (i32, i32) {
        let footprint_w = self.grid.tile_width as f32 * self.zoom;
        let footprint_h = footprint_w / 2.0;
        let canvas_h = self.grid.tile_height as f32 * self.zoom;
        let rel_x = mouse_x - self.origin_x;
        let rel_y = mouse_y - self.origin_y + (canvas_h - footprint_h); // undo the same shift draw() applies

        //let half_w = self.grid.tile_width as f32 / 2.0;
        //let half_h = self.grid.tile_height as f32 / 2.0;
        //let half_w = (self.grid.tile_width as f32 * self.zoom) / 2.0;
        //let half_h = (self.grid.tile_height as f32 * self.zoom) / 2.0;
        let half_w = (self.grid.tile_width as f32 * self.zoom) / 2.0;
        let half_h = half_w / 2.0;

        let tx = (rel_y / half_h + rel_x / half_w) / 2.0;
        let ty = (rel_y / half_h - rel_x / half_w) / 2.0;

        let selected_x = tx.floor() as i32;
        let selected_y = ty.floor() as i32;
        (selected_x, selected_y)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("transparent")
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.screen_width as f64,
                self.screen_height as f64,
            ));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );

        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());

        println!("w h: {} {}", self.screen_width, self.screen_height);
        let pixels = Pixels::new(window_size.width, window_size.height, surface_texture)
            .expect("failed to create pixels");

        self.screen_height = window_size.height;
        self.screen_width = window_size.width;
        self.origin_x = self.screen_width as f32 / 1.;
        self.origin_y = self.screen_height as f32 / 2.;
        self.window = Some(window);
        self.pixels = Some(pixels);

        let path = "/home/rabbit/Downloads/isometric tileset/sprites/train-r4.png";
        let train_sprite = load_sprite(path).unwrap();
        self.sprites = vec![train_sprite];
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    if let Err(err) = pixels.resize_surface(size.width, size.height) {
                        eprintln!("pixels.resize_surface error: {err}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                if event.state.is_pressed() {
                    match event.logical_key {
                        Key::Named(NamedKey::Backspace) => self.tile_n = 0,
                        Key::Named(NamedKey::Alt) => {
                            println!("Saving...");

                            let mut content = format!(
                                "{}\n{}\n{}\n{}\n",
                                self.grid.width,
                                self.grid.height,
                                self.grid.tile_width,
                                self.grid.tile_height
                            );

                            for tile in self.grid.tiles.iter() {
                                content.push_str("{");

                                for px in &tile.pxs {
                                    content.push_str(&format!(
                                        "[{}, {}, {}, {}]",
                                        px.0, px.1, px.2, px.3
                                    ));
                                }
                                content.push_str("}\n");
                            }
                            fs::write("tiles.txt", content).expect("Failed to save tiles");
                        }
                        Key::Character(c) => {
                            if let Some(n) = c.to_ascii_lowercase().parse::<u32>().ok() {
                                println!("n: {}", n);
                                println!(
                                    "10s: {} | {}",
                                    (self.tile_n.div_ceil(10) * 10),
                                    self.tile_n * (self.tile_n.div_ceil(10) * 10)
                                );
                                self.tile_n = self.tile_n * 10 + n
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01, // trackpads report pixels, scale down
                };

                //self.zoom += scroll_amount * 0.1; // tune sensitivity to taste
                //self.zoom = self.zoom.clamp(0.25, 4.0); // prevent zooming to zero or absurdly large

                let old_zoom = self.zoom;
                self.zoom = (self.zoom + scroll_amount * 0.1).clamp(0.25, 4.0);
                let zoom_ratio = self.zoom / old_zoom;

                // keep the point under the mouse stationary on screen
                self.origin_x = self.mouse_x - (self.mouse_x - self.origin_x as f32) * zoom_ratio;
                self.origin_y = self.mouse_y - (self.mouse_y - self.origin_y as f32) * zoom_ratio;

                //self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => match button {
                MouseButton::Left => {
                    let (selected_x, selected_y) = self.get_tile(self.mouse_x, self.mouse_y);

                    if selected_x >= 0
                        && selected_x < self.grid.width as i32
                        && selected_y >= 0
                        && selected_y < self.grid.height as i32
                    {
                        let idx = selected_y * self.grid.width as i32 + selected_x;
                        let dir = "/home/rabbit/Downloads/isometric tileset/separated images";
                        self.grid.tiles[idx as usize].pxs =
                            Tile::from_image(&format!("{}/tile_{:03}.png", dir, self.tile_n))
                                .unwrap()
                                .pxs;
                    }
                }
                _ => {}
            },
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;

                //let (selected_x, selected_y) = self.get_tile(self.mouse_x, self.mouse_y);

                //if selected_x >= 0
                //    && selected_x < self.grid.width as i32
                //    && selected_y >= 0
                //    && selected_y < self.grid.height as i32
                //{
                //    let idx = selected_y * self.grid.width as i32 + selected_x;
                //    self.grid.tiles[idx as usize].pxs = Tile::random(
                //        self.grid.tile_width as usize,
                //        self.grid.tile_height as usize,
                //    )
                //    .pxs;
                //}
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = self.draw() {
                    event_loop.exit();
                }
            }
            _ => {}
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
pub fn decode(data: &[f32]) -> String {
    let mut out = String::new();
    for d in data {
        out.push(char::from_u32(d.round() as u32).unwrap());
    }

    out
}
