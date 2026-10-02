use crate::{
    sprite::{Sprite, load_sprite},
    vec::Vec3,
};

pub struct Screen {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub sprites: Vec<Sprite>,
    pub tile_n: u32,
    pub space_pressed: bool,
    pub zoom: f32,
    pub holding_right: bool,
    pub target_zoom: f32,
    pub zoom_anchor: (f32, f32), // screen point that should stay fixed while zooming
    pub graphs: Vec<Vec<Vec3>>,
    pub bound_y: f32,
    pub follow_idx: usize,
    pub follow: bool,
    pub wind: Vec3,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            mouse_x: 0.,
            mouse_y: 0.,
            tile_n: 1,
            holding_right: false,
            space_pressed: false,
            origin_x: 0.,
            follow_idx: 0,
            follow: true,
            bound_y: 10000.,
            origin_y: 0.,
            offset_x: 0.,
            offset_y: 0.,
            zoom: 1.,
            sprites: vec![],
            target_zoom: 1.,
            graphs: vec![vec![]; 3 * 3],
            zoom_anchor: (0., 0.),
            wind: Vec3::from(0.0, 0.0, 0.),
        }
    }
}

impl Screen {
    pub fn init(&mut self, window_width: u32, window_height: u32) {
        self.origin_x = window_width as f32 / 1.;
        self.origin_y = window_height as f32 / 2.;
    }
    pub fn load_sprite(&mut self, path: &str, mass: f32, pos: Vec3) {
        let mut sprite = load_sprite(path, pos).unwrap();
        sprite.mass = mass;
        self.sprites.push(sprite);
    }
    pub fn load_sprites(&mut self, paths: &[&str]) {
        for path in paths {
            let sprite = load_sprite(path, Vec3::from(1., 1., 0.)).unwrap();
            self.sprites.push(sprite);
        }
        //let path = "/home/rabbit/Downloads/isometric tileset/sprites/leading-tr.png";
        //let train_sprite = load_sprite(path).unwrap();
        //self.sprites.push(train_sprite);

        //let path = "/home/rabbit/Downloads/isometric tileset/sprites/station5.png";
        //let train_sprite = load_sprite(path).unwrap();
        //self.sprites.push(train_sprite);

        //let path = "/home/rabbit/Downloads/isometric tileset/sprites/hopper.png";
        //let train_sprite = load_sprite(path).unwrap();
        //self.sprites.push(train_sprite);

        //let path = "/home/rabbit/Downloads/isometric tileset/sprites/carriage2.png";
        //let train_sprite = load_sprite(path).unwrap();
        //self.sprites.push(train_sprite);
    }
}
