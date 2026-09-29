use isometric::{grid::Grid, renderer::Renderer};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    const GRID_WIDTH: u32 = 32;
    const GRID_HEIGHT: u32 = 32;
    const TILE_WIDTH: u32 = 32;
    const TILE_HEIGHT: u32 = 32;
    const SCREEN_WIDTH: u32 = GRID_WIDTH * TILE_WIDTH;
    const SCREEN_HEIGHT: u32 = GRID_HEIGHT * TILE_HEIGHT;

    let grid = Grid::from_path("./tiles.txt");

    //let tiles_dir = "/home/rabbit/Downloads/isometric tileset/separated images";

    let font_data =
        include_bytes!("../../fonts/Carrois_Gothic_SC/CarroisGothicSC-Regular.ttf") as &[u8];
    let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
        .expect("failed to load font");

    let mut app = Renderer::init(grid, SCREEN_WIDTH, SCREEN_HEIGHT, font);

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("event loop error");
}
