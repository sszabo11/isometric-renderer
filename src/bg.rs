pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub fn from(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

pub fn draw_background(
    frame: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    screen_y: f32,
    bound_y: f32,
) {
    let bottom_color: Rgb = Rgb::from(22., 166., 255.);
    let mid_color: Rgb = Rgb::from(9., 18., 128.);
    let top_color: Rgb = Rgb::from(0., 0., 0.);

    let max_y = screen_y;
    let min_y = screen_y - screen_height as f32;

    for y in 0..screen_height {
        for x in 0..screen_width {
            let real_y = min_y + y as f32;
            let color = get_color_at_height(real_y, bound_y, &bottom_color, &mid_color, &top_color);
            let idx = y * screen_width + x;

            let offset = idx * 4;
            frame[offset..offset + 4].copy_from_slice(&[
                color.r.round() as u8,
                color.g.round() as u8,
                color.b.round() as u8,
                255,
            ]);
        }
    }
}

pub fn get_color_at_height(h: f32, max: f32, a: &Rgb, b: &Rgb, c: &Rgb) -> Rgb {
    let t = (h / max).clamp(0., 1.);

    let (local_p, c_1, c_2) = if t < 0.5 {
        (t * 2., a, b)
    } else {
        ((t - 0.5) * 2., b, c)
    };

    let r = c_1.r + local_p * (c_2.r - c_1.r);
    let g = c_1.g + local_p * (c_2.g - c_1.g);
    let b = c_1.b + local_p * (c_2.b - c_1.b);

    Rgb::from(r, g, b)
}
