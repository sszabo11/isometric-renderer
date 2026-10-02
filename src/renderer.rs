use std::{f32::consts::PI, fs, sync::Arc, time::Instant};

use anyhow::Result;
use fontdue::Font;
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
    bg::draw_background,
    grid::{Grid, Tile},
    hud::{draw_bank_gauge, draw_hud, flight_angles},
    screen::Screen,
    sprite::{Sprite, draw_sprite, load_sprite},
    vec::Vec3,
};

const AOA_RATE: f32 = 20.0; // degrees per second
const BANK_RATE: f32 = 30.0; // degrees per second
const TRIM_AOA: f32 = 30.0; // where AoA settles when released
pub struct Renderer {
    pub grid: Grid,
    pub window: Option<Arc<Window>>,
    pub pixels: Option<Pixels<'static>>,

    pub screen_width: u32,
    pub screen_height: u32,

    pub grid_tiles_width: u32,
    pub grid_tiles_height: u32,

    pub font: fontdue::Font,

    pub screen: Screen,
    //pub world: World,
    pub last_frame: std::time::Instant,
    pub tick: u32,

    pub accumulator: f32,
    pub sim_time: f32,
    pub time_warp: f32,
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

pub fn draw_text(
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
impl Renderer {
    pub fn init(grid: Grid, screen_width: u32, screen_height: u32, font: Font) -> Self {
        Self {
            grid,
            pixels: None,
            window: None,
            grid_tiles_width: screen_width,
            grid_tiles_height: screen_height,
            screen_width: screen_width,
            screen_height: screen_height,
            font,
            screen: Screen::default(),
            tick: 0,
            last_frame: Instant::now(),
            accumulator: 0.,
            time_warp: 1.,
            sim_time: 0.,
        }
    }

    fn follow_camera(&mut self) {
        if !self.screen.follow {
            return;
        }
        let Some(s) = self.screen.sprites.get(self.screen.follow_idx) else {
            return;
        };

        let zoom = self.screen.zoom;
        let fw = self.grid.tile_width as f32 * zoom; // same footprint math as draw_sprite
        let fh = fw / 2.0;
        let hs = 2.0; // same height scale as draw_sprite

        let sw = s.width as f32 * zoom;
        let sh = s.height as f32 * zoom;

        // Where the sprite's center sits relative to the origin
        let off_x = (s.pos.x - s.pos.y) * (fw / 2.0) + sw / 2.0;
        let off_y = (s.pos.x + s.pos.y) * (fh / 2.0) - s.pos.z * hs * zoom + sh / 2.0;

        self.screen.origin_x = self.screen_width as f32 / 2.0 - off_x;
        self.screen.origin_y = self.screen_height as f32 / 2.0 - off_y;
    }
    pub fn draw(&mut self) -> Result<(), Error> {
        self.follow_camera();
        let zoom = self.screen.zoom;
        let origin_x = self.screen.origin_x;
        let origin_y = self.screen.origin_y;
        let mouse_x = self.screen.mouse_x;
        let mouse_y = self.screen.mouse_y;

        let tile_width = (self.grid.tile_width as f32 * zoom) as usize;
        let tile_height = (self.grid.tile_height as f32 * zoom) as usize;

        let screen_width = self.screen_width as usize;
        let screen_height = self.screen_height as usize;

        let base_w = self.grid.tile_width as usize;
        let base_h = self.grid.tile_height as usize;

        let footprint_w = base_w as f32 * zoom;
        let footprint_h = footprint_w / 2.0;

        let canvas_w = footprint_w;
        let canvas_h = base_h as f32 * zoom;

        let x_start = origin_x as isize;
        let y_start = origin_y as isize;

        let (selected_x, selected_y) = self.get_tile(mouse_x, mouse_y);
        if let Some(pixels) = self.pixels.as_mut() {
            let frame = pixels.frame_mut();
            frame.fill(0);
            draw_background(
                frame,
                screen_width,
                screen_height,
                self.screen.origin_y,
                self.screen.bound_y,
            );

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
                            let src_x = ((px as f32 / zoom) as usize).min(base_w - 1);
                            let src_y = ((py as f32 / zoom) as usize).min(base_h - 1);

                            let hs = 2;
                            let tile_px = tile.pxs[src_y * self.grid.tile_width as usize + src_x];
                            let z = tile.z;
                            let screen_x = iso_x + px as isize;
                            let screen_y = iso_y + py as isize - (z * hs) as isize;

                            if screen_x < 0
                                || screen_y < 0
                                || screen_x as usize >= screen_width
                                || screen_y as usize >= screen_height
                            {
                                continue;
                            }

                            let pixel_idx = screen_y as usize * screen_width + screen_x as usize;
                            let offset = pixel_idx * 4;

                            if tile_px.3 == 0 {
                                continue;
                            }
                            frame[offset..offset + 4]
                                .copy_from_slice(&[tile_px.0, tile_px.1, tile_px.2, tile_px.3]);
                        }
                    }
                }
            }
            if self.screen.sprites.len() > 0 {
                for (i, sprite) in self.screen.sprites.iter_mut().enumerate() {
                    //if self.screen.space_pressed {
                    //    falling(self.tick, sprite, self.screen.wind);
                    //    //println!("i: {}", i);
                    //    //println!("{:?}", self.screen.graphs);
                    //    //println!("{:?}", self.screen.graphs[0]);

                    //    if self.tick % 1 == 0 {
                    //        let base = i * 3;
                    //        self.screen.graphs[base + 0].push(sprite.vel);
                    //        self.screen.graphs[base + 1].push(sprite.acc);
                    //        self.screen.graphs[base + 2].push(sprite.pos);
                    //    }

                    //    if sprite.pos.z == 0. && !sprite.flag {
                    //        println!("Saving...");
                    //        let base = i * 3;
                    //        let v_graph: &Vec<f32> = &self.screen.graphs[base]
                    //            .iter()
                    //            .map(|v| v.z)
                    //            .collect::<Vec<f32>>();
                    //        let acc_graph = &self.screen.graphs[base + 1]
                    //            .iter()
                    //            .map(|v| v.z)
                    //            .collect::<Vec<f32>>();
                    //        let z_graph = &self.screen.graphs[base + 2]
                    //            .iter()
                    //            .map(|v| v.z)
                    //            .collect::<Vec<f32>>();

                    //        //simple_plot::plot!(
                    //        //    &format!("Velocity vs Time for {}kg", sprite.mass),
                    //        //    v_graph
                    //        //);
                    //        //simple_plot::plot!(
                    //        //    &format!("Acceleration vs Time for {}kg", sprite.mass),
                    //        //    acc_graph
                    //        //);
                    //        //simple_plot::plot!(
                    //        //    &format!("Height vs Time for {}kg", sprite.mass),
                    //        //    z_graph
                    //        //);
                    //        sprite.flag = true;
                    //    }
                    //}
                    draw_sprite(
                        frame,
                        screen_width,
                        screen_height,
                        origin_x,
                        origin_y,
                        zoom,
                        self.grid.tile_width,
                        sprite.pos,
                        &sprite,
                    );
                }
            }
            if let Some(s) = self.screen.sprites.get(0) {
                let angles = flight_angles(s.vel - self.screen.wind, s.angle_attack);
                draw_hud(
                    frame,
                    screen_width,
                    screen_height,
                    &self.font,
                    &angles,
                    screen_width as i32 - 280,
                    150,
                );
                draw_bank_gauge(
                    frame,
                    screen_width,
                    screen_height,
                    &self.font,
                    s.bank_angle,
                    screen_width as i32 - 280,
                    150 + 295,
                );
            }
            //draw_sprite(
            //    frame,
            //    screen_width,
            //    screen_height,
            //    origin_x,
            //    origin_y,
            //    zoom,
            //    self.grid.tile_width,
            //    0.,
            //    6.,
            //    &self.screen.sprites[1],
            //);
            //draw_sprite(
            //    frame,
            //    screen_width,
            //    screen_height,
            //    origin_x,
            //    origin_y,
            //    zoom,
            //    self.grid.tile_width,
            //    6. + self.tick as f32 / 100.,
            //    10.4,
            //    &self.screen.sprites[3],
            //);
            //draw_sprite(
            //    frame,
            //    screen_width,
            //    screen_height,
            //    origin_x,
            //    origin_y,
            //    zoom,
            //    self.grid.tile_width,
            //    6. + self.tick as f32 / 100.,
            //    10.4,
            //    &self.screen.sprites[0],
            //);
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Time warp: {}x", self.time_warp),
                (self.screen_width - 300) as i32,
                60,
                36.0,
                [255, 0, 0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Sim time: {:.2}", self.sim_time),
                (self.screen_width - 300) as i32,
                100,
                36.0,
                [255, 0, 0],
            );
            let sprite = &self.screen.sprites[0];
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!(
                    "X-vel: {:.2} | Y-vel: {:.2} | Z-vel: {:.2}",
                    sprite.vel.x, sprite.vel.y, sprite.vel.z
                ),
                60,
                (self.screen_height - 100) as i32,
                36.0,
                [255, 0, 0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Tile number: {}", self.screen.tile_n),
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
                &format!("Mouse: ({:.2}, {:.2})", mouse_x, mouse_y),
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
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Acceleration: {:.2}", self.screen.sprites[0].acc.z),
                100,
                180,
                36.0,
                [255, 0, 0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Velocity: {:.2}", self.screen.sprites[0].vel.z),
                100,
                220,
                36.0,
                [255, 0, 0],
            );
            draw_text(
                frame,
                screen_width,
                screen_height,
                &self.font,
                &format!("Bank angle: {:.2}", self.screen.sprites[0].bank_angle),
                100,
                260,
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
        let zoom = self.screen.zoom;
        let origin_x = self.screen.origin_x;
        let origin_y = self.screen.origin_y;
        let mouse_x = self.screen.mouse_x;
        let mouse_y = self.screen.mouse_y;

        let footprint_w = self.grid.tile_width as f32 * zoom;
        let footprint_h = footprint_w / 2.0;
        let canvas_h = self.grid.tile_height as f32 * zoom;
        let rel_x = mouse_x - origin_x;
        let rel_y = mouse_y - origin_y + (canvas_h - footprint_h); // undo the same shift draw() applies

        let half_w = (self.grid.tile_width as f32 * zoom) / 2.0;
        let half_h = half_w / 2.0;

        let tx = (rel_y / half_h + rel_x / half_w) / 2.0;
        let ty = (rel_y / half_h - rel_x / half_w) / 2.0;

        let selected_x = tx.floor() as i32;
        let selected_y = ty.floor() as i32;
        (selected_x, selected_y)
    }
}

impl ApplicationHandler for Renderer {
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
        self.window = Some(window);
        self.pixels = Some(pixels);

        self.screen.init(window_size.width, window_size.height);
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
                match &event.logical_key {
                    Key::Character(c) => {
                        match c.to_ascii_lowercase().chars().next().unwrap() {
                            'a' => {
                                if event.state.is_pressed() {
                                    self.screen.sprites[0].banking = -1;
                                } else {
                                    self.screen.sprites[0].banking = 0;
                                }
                                //self.screen.sprites[0].bank_angle -= 1.;
                                //self.screen.sprites[0].bank_angle.clamp(-180, max)
                            }
                            'd' => {
                                if event.state.is_pressed() {
                                    self.screen.sprites[0].banking = 1;
                                } else {
                                    self.screen.sprites[0].banking = 0;
                                }
                                //self.screen.sprites[0].bank_angle += 1.;
                            }
                            's' => {
                                if event.state.is_pressed() {
                                    self.screen.sprites[0].pitching = -1;
                                } else {
                                    self.screen.sprites[0].pitching = 0;
                                }
                                //self.screen.sprites[0].bank_angle -= 1.;
                                //self.screen.sprites[0].bank_angle.clamp(-180, max)
                            }
                            'w' => {
                                if event.state.is_pressed() {
                                    self.screen.sprites[0].pitching = 1;
                                } else {
                                    self.screen.sprites[0].pitching = 0;
                                }
                                //self.screen.sprites[0].bank_angle += 1.;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                if event.state.is_pressed() {
                    match event.logical_key {
                        Key::Named(NamedKey::Backspace) => self.screen.tile_n = 0,
                        Key::Named(NamedKey::Space) => self.screen.space_pressed = true,
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
                            fs::write("space.txt", content).expect("Failed to save tiles");
                        }
                        Key::Character(c) => {
                            match c.to_ascii_lowercase().chars().next().unwrap() {
                                //'a' => {
                                //    self.screen.sprites[0].banking = -1;
                                //    //self.screen.sprites[0].bank_angle -= 1.;
                                //    //self.screen.sprites[0].bank_angle.clamp(-180, max)
                                //}
                                //'d' => {
                                //    self.screen.sprites[0].banking = 1;
                                //    //self.screen.sprites[0].bank_angle += 1.;
                                //}
                                'f' => {
                                    self.screen.follow = !self.screen.follow;
                                }
                                ',' => {
                                    let warp = self.time_warp - 1.;
                                    self.time_warp = warp.clamp(0., 100.);
                                }
                                '.' => {
                                    let warp = self.time_warp + 1.;
                                    self.time_warp = warp.clamp(0., 100.);
                                }
                                _ => {}
                            }
                            if let Some(n) = c.to_ascii_lowercase().parse::<u32>().ok() {
                                println!("n: {}", n);
                                println!(
                                    "10s: {} | {}",
                                    (self.screen.tile_n.div_ceil(10) * 10),
                                    self.screen.tile_n * (self.screen.tile_n.div_ceil(10) * 10)
                                );
                                self.screen.tile_n = self.screen.tile_n * 10 + n
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                //let scroll_amount = match delta {
                //    MouseScrollDelta::LineDelta(_, y) => y,
                //    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01, // trackpads report pixels, scale down
                //};
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
                };
                let old_zoom = self.screen.zoom;
                self.screen.zoom = (self.screen.zoom + scroll * 0.1).clamp(0.01, 7.0);
                if !self.screen.follow {
                    let zoom_ratio = self.screen.zoom / old_zoom;

                    if self.screen.zoom < 0.1 {
                        self.screen.origin_x = self.screen.mouse_x
                            - (self.screen.mouse_x - self.screen.origin_x) * zoom_ratio / 100.;
                        self.screen.origin_y = self.screen.mouse_y
                            - (self.screen.mouse_y - self.screen.origin_y) * zoom_ratio / 100.;
                    } else {
                        // keep the point under the mouse stationary on screen
                        self.screen.origin_x = self.screen.mouse_x
                            - (self.screen.mouse_x - self.screen.origin_x) * zoom_ratio;
                        self.screen.origin_y = self.screen.mouse_y
                            - (self.screen.mouse_y - self.screen.origin_y) * zoom_ratio;
                    }
                }
            }

            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => match button {
                MouseButton::Right => {
                    if state.is_pressed() {
                        self.screen.offset_x = self.screen.mouse_x - self.screen.origin_x;
                        self.screen.offset_y = self.screen.mouse_y - self.screen.origin_y;
                        self.screen.holding_right = true;
                    } else {
                        self.screen.holding_right = false;
                    }
                }
                MouseButton::Left => {
                    let (selected_x, selected_y) =
                        self.get_tile(self.screen.mouse_x, self.screen.mouse_y);

                    if selected_x >= 0
                        && selected_x < self.grid.width as i32
                        && selected_y >= 0
                        && selected_y < self.grid.height as i32
                    {
                        let idx = selected_y * self.grid.width as i32 + selected_x;
                        let dir = "/home/rabbit/Downloads/isometric tileset/separated images";
                        self.grid.tiles[idx as usize].pxs = Tile::from_image(&format!(
                            "{}/tile_{:03}.png",
                            dir, self.screen.tile_n
                        ))
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
                self.screen.mouse_x = position.x as f32;
                self.screen.mouse_y = position.y as f32;

                if self.screen.holding_right {
                    self.screen.origin_x = self.screen.mouse_x - self.screen.offset_x;
                    self.screen.origin_y = self.screen.mouse_y - self.screen.offset_y;
                }
            }
            WindowEvent::RedrawRequested => {
                const DT: f32 = 0.004; // fixed physics step: 250 Hz
                const MAX_STEPS: u32 = 2000; // safety cap per frame

                let now = std::time::Instant::now();
                let frame_dt = (now - self.last_frame).as_secs_f32().min(0.1);
                self.last_frame = now;

                self.accumulator += frame_dt * self.time_warp;

                //let sprite = &mut self.screen.sprites[0];
                //if sprite.banking > 0 {
                //    sprite.bank_angle += 1.;
                //} else if sprite.banking < 0 {
                //    sprite.bank_angle -= 1.;
                //} else {
                //    if sprite.bank_angle > 0. {
                //        sprite.bank_angle -= 1.;
                //    } else {
                //        sprite.bank_angle += 1.;
                //    }
                //}

                //if sprite.pitching > 0 {
                //    sprite.angle_attack += 1.;
                //} else if sprite.pitching < 0 {
                //    sprite.angle_attack -= 1.;
                //} else {
                //    if sprite.angle_attack > 30. {
                //        sprite.angle_attack -= 1.;
                //    } else {
                //        sprite.angle_attack += 1.;
                //    }
                //}

                let mut steps = 0;
                while self.accumulator >= DT && steps < MAX_STEPS {
                    if self.screen.sprites.len() > 0 && self.screen.space_pressed {
                        for (i, sprite) in self.screen.sprites.iter_mut().enumerate() {
                            update_controls(sprite, DT);
                            if self.screen.space_pressed {
                                falling(DT, sprite, self.screen.wind);

                                if self.tick % 1 == 0 && !sprite.landed {
                                    let base = i * 3;
                                    self.screen.graphs[base + 0].push(sprite.vel);
                                    self.screen.graphs[base + 1].push(sprite.acc);
                                    self.screen.graphs[base + 2].push(sprite.pos);
                                }

                                if sprite.pos.z == 0. && !sprite.flag {
                                    println!("Saving...");
                                    let base = i * 3;
                                    let v_graph: &Vec<f32> = &self.screen.graphs[base]
                                        .iter()
                                        .map(|v| v.z)
                                        .collect::<Vec<f32>>();
                                    let acc_graph = &self.screen.graphs[base + 1]
                                        .iter()
                                        .map(|v| v.z)
                                        .collect::<Vec<f32>>();
                                    let z_graph = &self.screen.graphs[base + 2]
                                        .iter()
                                        .map(|v| v.z)
                                        .collect::<Vec<f32>>();

                                    sprite.flag = true;
                                }
                            }
                        }
                    }
                    //falling(DT, &mut self.screen.sprites[0], wind); // add a loop for each sprite
                    self.accumulator -= DT;
                    self.sim_time += DT;
                    steps += 1;
                }
                if steps == MAX_STEPS {
                    self.accumulator = 0.0; // drop the backlog instead of spiraling
                }
                if self.draw().is_err() {
                    event_loop.exit();
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn falling(dt: f32, sprite: &mut Sprite, wind: Vec3) {
    //let dt = tick as f32 / 1000.;

    let gravity = Vec3::from(0., 0., -9.8 * sprite.mass);

    let p: f32 = 1.225; // Density of fluid
    let a = 1.; // surface area
    //let cd = 2.0; // Drag  coeefficent
    //let cl = 1.4; // Lift coeefficent
    //let alpha = sprite.angle_attack.to_radians(); // aoa stored in degrees, like bank_angle
    let alpha = sprite.angle_attack.clamp(0.0, 90.0).to_radians();
    let cn_max = 2.0; // normal-force coefficient at 90°
    let cd0 = 0.05; // baseline drag

    let cn = cn_max * alpha.sin().powi(2);
    let cl = cn * alpha.cos();
    let cd = cd0 + cn * alpha.sin();

    let rel_v = sprite.vel - wind;

    let speed = rel_v.magnitude();
    let drag = rel_v * (-0.5 * p * cd * a * speed);

    let mut lift_force = Vec3::from(0., 0., 0.);
    if speed > 1e-3 {
        let v_hat = rel_v / speed;

        let mut reference = Vec3::from(0., 0., 1.);
        let mut u_raw = reference - v_hat * reference.dot(v_hat);
        if u_raw.magnitude() < 1e-3 {
            reference = Vec3::from(1., 0., 0.);
            u_raw = reference - v_hat * reference.dot(v_hat);
        }

        let u_hat = u_raw / u_raw.magnitude();

        let w_hat = v_hat.cross(u_hat);

        let lift_mag = 0.5 * p * cl * a * speed * speed;
        let phi = sprite.bank_angle.to_radians();
        let lift_dir = u_hat * phi.cos() + w_hat * phi.sin();
        //let lift_dir = u_hat * (sprite.bank_angle.cos() * (PI / 180.)) + w_hat * sprite.bank_angle.sin() * (PI / 180.);
        lift_force = lift_dir * lift_mag;
    }

    let acc = (gravity + drag + lift_force) / sprite.mass;

    if !sprite.landed {
        sprite.acc = acc;

        sprite.vel += acc * dt;

        sprite.pos += sprite.vel * dt;
        if sprite.pos.z <= 0. {
            sprite.pos.z = 0.;
            sprite.vel = Vec3::from(0., 0., 0.);
            sprite.acc = Vec3::from(0., 0., 0.);
            sprite.landed = true;
        }
    }
}
fn move_toward(cur: f32, target: f32, max_delta: f32) -> f32 {
    let d = target - cur;
    if d.abs() <= max_delta {
        target
    } else {
        cur + d.signum() * max_delta
    }
}
fn update_controls(s: &mut Sprite, dt: f32) {
    if s.pitching > 0 {
        s.angle_attack += AOA_RATE * dt;
    } else if s.pitching < 0 {
        s.angle_attack -= AOA_RATE * dt;
    } else {
        s.angle_attack = move_toward(s.angle_attack, TRIM_AOA, AOA_RATE * dt);
    }
    s.angle_attack = s.angle_attack.clamp(0.0, 90.0);

    if s.banking > 0 {
        s.bank_angle += BANK_RATE * dt;
    } else if s.banking < 0 {
        s.bank_angle -= BANK_RATE * dt;
    }
    // keep the stored value in -180..180
    s.bank_angle = (s.bank_angle + 180.0).rem_euclid(360.0) - 180.0;
}
