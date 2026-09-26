use std::sync::Arc;

use anyhow::Result;
use pixels::{Error, Pixels, SurfaceTexture};
use rand::{RngExt, SeedableRng, rand_core::block::Generator};
use winit::{
    application::ApplicationHandler,
    event::{Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::Key,
    window::{Window, WindowId},
};

use crate::{Grid, Tile};

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
    pub origin_x: u32,
    pub origin_y: u32,
}

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
        let tile_width = self.grid.tile_width as usize;
        let tile_height = self.grid.tile_height as usize;
        let screen_width = self.screen_width as usize;
        let screen_height = self.screen_height as usize;

        let x_start = self.origin_x;
        let y_start = self.origin_y;

        let (selected_x, selected_y) = self.get_tile(self.mouse_x, self.mouse_y);
        if let Some(pixels) = self.pixels.as_mut() {
            let frame = pixels.frame_mut();

            // Get tile place
            for ty in 0..self.grid.height as usize {
                for tx in 0..self.grid.width as usize {
                    let tile_idx = ty * self.grid.width as usize + tx;
                    let tile = &self.grid.tiles[tile_idx];

                    let iso_x =
                        x_start as isize + (tx as isize - ty as isize) * (tile_width as isize / 2);
                    let iso_y =
                        y_start as isize + (tx as isize + ty as isize) * (tile_height as isize / 2);

                    // Fill inside tile
                    for py in 0..tile_height {
                        for px in 0..tile_width {
                            let screen_x = iso_x + px as isize;
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

                            let tile_px = tile.pxs[py * tile_width + px];

                            if tile_px.3 == 0 {
                                continue;
                            }
                            frame[offset..offset + 4]
                                .copy_from_slice(&[tile_px.0, tile_px.1, tile_px.2, tile_px.3]);
                        }
                    }
                }
            }
            for y in 0..500 {
                for x in 0..600 {
                    let idx = (y * screen_width + x) * 4;
                    frame[idx..idx + 4].copy_from_slice(&[0, 0, 0, 0]);
                }
            }
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
        let rel_x = mouse_x - self.origin_x as f32;
        let rel_y = mouse_y - self.origin_y as f32;

        let half_w = self.grid.tile_width as f32 / 2.0;
        let half_h = self.grid.tile_height as f32 / 2.0;

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
        self.origin_x = self.screen_width / 2;
        self.origin_y = self.screen_height / 4;
        println!("ogx: {} ogt: {}", self.origin_x, self.origin_y);
        self.window = Some(window);
        self.pixels = Some(pixels);
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
            } => match event.logical_key {
                Key::Character(c) => {
                    let idx = c.to_ascii_lowercase().chars().next().unwrap() as u32;
                }
                _ => {}
            },
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;

                let (selected_x, selected_y) = self.get_tile(self.mouse_x, self.mouse_y);

                if selected_x >= 0
                    && selected_x < self.grid.width as i32
                    && selected_y >= 0
                    && selected_y < self.grid.height as i32
                {
                    let idx = selected_y * self.grid.width as i32 + selected_x;
                    self.grid.tiles[idx as usize].pxs = Tile::random(
                        self.grid.tile_width as usize,
                        self.grid.tile_height as usize,
                    )
                    .pxs;
                }
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
