use anyhow::Result;
use image::{GenericImageView, ImageReader};
use rand::RngExt;

use crate::renderer::read_tile_file;

pub type Px = (u8, u8, u8, u8);

#[derive(Debug, Clone)]
pub struct Tile {
    pub pxs: Vec<Px>,
    pub z: u32,
}

impl Tile {
    pub fn empty(w: usize, h: usize) -> Self {
        Self {
            pxs: vec![(0, 0, 0, 0); w * h],
            z: 0,
        }
    }

    pub fn from_image(path: &str) -> Result<Self> {
        let img = ImageReader::open(path)?.decode()?;
        let pxs = img
            .pixels()
            .map(|(_x, _y, color)| (color[0], color[1], color[2], color[3]))
            .collect();

        Ok(Self { pxs: pxs, z: 0 })
    }

    pub fn random(w: usize, h: usize) -> Self {
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

        Self { pxs, z: 0 }
    }
}

pub struct Grid {
    pub tiles: Vec<Tile>,
    pub tile_width: u32,
    pub tile_height: u32,
    pub width: u32,
    pub height: u32,
}
impl Grid {
    pub fn empty(width: u32, height: u32, tile_w: u32, tile_h: u32) -> Self {
        Self {
            width,
            tile_width: tile_w,
            tile_height: tile_h,
            height,
            tiles: vec![Tile::empty(tile_w as usize, tile_h as usize); (width * height) as usize],
        }
    }

    pub fn default(width: u32, height: u32, tile_w: u32, tile_h: u32) -> Self {
        //let dir = "/home/rabbit/Downloads/isometric tileset/separated images";
        //let tile = Tile::from_image(&format!("{}/tile_000.png", dir)).unwrap();
        let dir = "/home/rabbit/Downloads/isometric tileset/128x128";
        let tile = Tile::from_image(&format!("{}/grass.png", dir)).unwrap();
        Self {
            width,
            tile_width: tile_w,
            tile_height: tile_h,
            height,
            tiles: vec![tile; (width * height) as usize],
        }
    }

    pub fn from_path(path: &str) -> Self {
        let (tiles, tile_w, tile_h, grid_w, grid_h) = read_tile_file(path);

        let mut g = Self::empty(grid_w, grid_h, tile_w, tile_h);
        for y in 0..grid_h {
            for x in 0..grid_w {
                let i = y * grid_w + x;
                g.set_tile(x, y, tiles[i as usize].pxs.clone());
            }
        }
        g
    }

    pub fn set_tile(&mut self, x: u32, y: u32, pxs: Vec<Px>) {
        let i = (y * self.width + x) as usize;
        self.tiles[i].pxs = pxs;
    }
}
