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
}

impl App {
    pub fn draw(&mut self) -> Result<(), Error> {
        let tile_width = self.grid.tile_width as usize;
        let tile_height = self.grid.tile_height as usize;
        let screen_width = self.screen_width as usize;
        let screen_height = self.screen_height as usize;

        let x_start = screen_width / 2;
        let y_start = screen_height / 4;

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
                            //let screen_x = tx * tile_width + px;
                            //let screen_y = ty * tile_height + py;
                            //let screen_x = x_start + tx * (tile_width / 2) + px;
                            //let screen_y = y_start + tx * (tile_height / 2) + py;
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
                            //let pixel_idx = screen_y * screen_width + screen_x;

                            //let offset = pixel_idx * 4;

                            let tile_px = tile.pxs[py * tile_width + px];

                            //if px == 0
                            //    || py == 0
                            //    || px == tile_width as usize
                            //    || py == tile_height as usize
                            //{
                            //    frame[offset..offset + 4].copy_from_slice(&[0, 0, 255, 255]);
                            //} else {
                            if tile_px.3 == 0 {
                                continue;
                            }
                            frame[offset..offset + 4]
                                .copy_from_slice(&[tile_px.0, tile_px.1, tile_px.2, tile_px.3]);
                            //}
                        }
                    }
                }
            }
            if let Err(err) = pixels.render() {
                eprintln!("pixels.render error: {err}");
                return Err(err);
            }
        }

        Ok(())
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
        let pixels = Pixels::new(self.screen_width, self.screen_height, surface_texture)
            .expect("failed to create pixels");

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
