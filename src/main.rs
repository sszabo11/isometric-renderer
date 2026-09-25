use std::fs;

use anyhow::Result;
use image::{GenericImageView, ImageReader};
use rand::RngExt;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window,
};

use crate::app::App;

mod app;

type Px = (u8, u8, u8, u8);

#[derive(Debug, Clone)]
struct Tile {
    pxs: Vec<Px>,
}

impl Tile {
    fn empty(w: usize, h: usize) -> Self {
        Self {
            pxs: vec![(0, 0, 0, 255); w * h],
        }
    }

    fn from_image(path: &str) -> Result<Self> {
        let img = ImageReader::open(path)?.decode()?;
        let pxs = img
            .pixels()
            .map(|(_x, _y, color)| (color[0], color[1], color[2], color[3]))
            .collect();

        Ok(Self { pxs: pxs })
    }

    fn random(w: usize, h: usize) -> Self {
        let mut rng = rand::rng();
        let pxs = (0..w * h)
            .map(|_| {
                let r = rng.random_range(0..255);
                let g = rng.random_range(0..255);
                let b = rng.random_range(0..255);
                let a = rng.random_range(0..255);
                (r, g, b, a)
            })
            .collect();

        Self { pxs }
    }
}

struct Grid {
    tiles: Vec<Tile>,
    tile_width: u32,
    tile_height: u32,
    width: u32,
    height: u32,
}
impl Grid {
    fn empty(width: u32, height: u32, tile_w: u32, tile_h: u32) -> Self {
        Self {
            width,
            tile_width: tile_w,
            tile_height: tile_h,
            height,
            tiles: vec![Tile::empty(tile_w as usize, tile_h as usize); (width * height) as usize],
        }
    }

    fn set_tile(&mut self, x: u32, y: u32, pxs: Vec<Px>) {
        let i = (y * self.width + x) as usize;
        self.tiles[i].pxs = pxs;
    }
}

fn main() {
    const GRID_WIDTH: u32 = 128;
    const GRID_HEIGHT: u32 = 128;
    const TILE_WIDTH: u32 = 32;
    const TILE_HEIGHT: u32 = 32;
    const SCREEN_WIDTH: u32 = GRID_WIDTH * TILE_WIDTH;
    const SCREEN_HEIGHT: u32 = GRID_HEIGHT * TILE_HEIGHT;

    let mut grid = Grid::empty(GRID_WIDTH, GRID_HEIGHT, TILE_WIDTH, TILE_HEIGHT);

    let tiles_dir =
        "/home/rabbit/Downloads/Forest_Isometric_Pack_Free/Forest Isometric Pack Free/Tileset";
    let files = fs::read_dir(tiles_dir).unwrap();
    let paths: Vec<_> = files.map(|f| f.unwrap().path()).collect();

    let mut rng = rand::rng();
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let idx = rng.random_range(0..paths.len());

            grid.set_tile(
                x,
                y,
                Tile::from_image(paths[idx].to_str().unwrap()).unwrap().pxs,
            );
        }
    }

    let mut app = App {
        grid,
        pixels: None,
        window: None,
        screen_width: SCREEN_WIDTH,
        screen_height: SCREEN_HEIGHT,
    };

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("event loop error");
}
