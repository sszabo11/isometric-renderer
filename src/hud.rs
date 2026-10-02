use fontdue::Font;

use crate::{renderer::draw_text, vec::Vec3};

pub struct FlightAngles {
    pub speed: f32,
    pub path: f32,            // flight path angle, degrees, + = climbing
    pub heading: Option<f32>, // degrees from +x toward +y; None if nearly vertical
    pub aoa: f32,
    pub pitch: f32, // nose angle above horizon = path + aoa
}

pub fn flight_angles(air_v: Vec3, aoa_deg: f32) -> FlightAngles {
    let speed = air_v.magnitude();
    let horiz = (air_v.x * air_v.x + air_v.y * air_v.y).sqrt();

    let path = if speed > 0.1 {
        air_v.z.atan2(horiz).to_degrees()
    } else {
        0.0
    };
    let heading = if horiz > 0.1 {
        Some(air_v.y.atan2(air_v.x).to_degrees().rem_euclid(360.0))
    } else {
        None // heading is undefined when falling straight down
    };

    FlightAngles {
        speed,
        path,
        heading,
        aoa: aoa_deg,
        pitch: path + aoa_deg,
    }
}

// ---------- tiny drawing helpers ----------

fn blend_px(frame: &mut [u8], w: usize, h: usize, x: i32, y: i32, c: [u8; 3], a: f32) {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        return;
    }
    let o = (y as usize * w + x as usize) * 4;
    for i in 0..3 {
        frame[o + i] = (frame[o + i] as f32 * (1.0 - a) + c[i] as f32 * a) as u8;
    }
    frame[o + 3] = 255;
}

fn fill_rect(
    frame: &mut [u8],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    rw: i32,
    rh: i32,
    c: [u8; 3],
    a: f32,
) {
    for yy in y..y + rh {
        for xx in x..x + rw {
            blend_px(frame, w, h, xx, yy, c, a);
        }
    }
}

// Bresenham line
fn draw_line(frame: &mut [u8], w: usize, h: usize, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 3]) {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        blend_px(frame, w, h, x, y, c, 1.0);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

// Point at radius r and angle `deg` from the center. 0 deg = right, positive = up
// (screen y points down, hence the minus).
fn polar(cx: f32, cy: f32, r: f32, deg: f32) -> (i32, i32) {
    let a = deg.to_radians();
    (
        (cx + r * a.cos()).round() as i32,
        (cy - r * a.sin()).round() as i32,
    )
}

fn draw_arc(
    frame: &mut [u8],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    r: f32,
    a0: f32,
    a1: f32,
    c: [u8; 3],
) {
    let steps = ((a1 - a0).abs().to_radians() * r * 2.0).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * (i as f32 / steps as f32);
        let (x, y) = polar(cx, cy, r, a);
        blend_px(frame, w, h, x, y, c, 1.0);
    }
}

fn draw_arrow(
    frame: &mut [u8],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    r: f32,
    deg: f32,
    c: [u8; 3],
) {
    let (tx, ty) = polar(cx, cy, r, deg);
    draw_line(frame, w, h, cx as i32, cy as i32, tx, ty, c);
    for barb in [150.0, -150.0] {
        let (bx, by) = polar(tx as f32, ty as f32, 8.0, deg + barb);
        draw_line(frame, w, h, tx, ty, bx, by, c);
    }
}

// ---------- the HUD ----------

pub fn draw_hud(
    frame: &mut [u8],
    w: usize,
    h: usize,
    font: &Font,
    a: &FlightAngles,
    x0: i32,
    y0: i32,
) {
    const PANEL_W: i32 = 260;
    const PANEL_H: i32 = 285;
    const GREEN: [u8; 3] = [80, 255, 120];
    const ORANGE: [u8; 3] = [255, 170, 40];
    const GREY: [u8; 3] = [110, 110, 140];

    fill_rect(frame, w, h, x0, y0, PANEL_W, PANEL_H, [10, 10, 25], 0.6);

    // --- side view ---
    let cx = (x0 + 130) as f32;
    let cy = (y0 + 80) as f32;
    let r = 60.0;

    // horizon
    draw_line(
        frame,
        w,
        h,
        x0 + 10,
        cy as i32,
        x0 + PANEL_W - 10,
        cy as i32,
        GREY,
    );

    // motion vector (green arrow)
    draw_arrow(frame, w, h, cx, cy, r, a.path, GREEN);

    // nose axis (orange line, with a short tail behind the center)
    let (tail_x, tail_y) = polar(cx, cy, r * 0.5, a.pitch + 180.0);
    let (nose_x, nose_y) = polar(cx, cy, r * 0.95, a.pitch);
    draw_line(frame, w, h, tail_x, tail_y, nose_x, nose_y, ORANGE);

    // AoA arc between the two
    draw_arc(frame, w, h, cx, cy, r * 0.45, a.path, a.pitch, ORANGE);

    draw_text(frame, w, h, font, "Motion", x0 + 10, y0 + 6, 18.0, GREEN);
    draw_text(
        frame,
        w,
        h,
        font,
        "Nose",
        x0 + PANEL_W - 60,
        y0 + 6,
        18.0,
        ORANGE,
    );

    // --- heading compass (world axes: +x right, +y up) ---
    let ccx = (x0 + 195) as f32;
    let ccy = (y0 + 215) as f32;
    let cr = 36.0;
    draw_arc(frame, w, h, ccx, ccy, cr, 0.0, 360.0, GREY);
    if let Some(hd) = a.heading {
        draw_arrow(frame, w, h, ccx, ccy, cr - 2.0, hd, GREEN);
    }

    // --- numbers ---
    let heading_text = match a.heading {
        Some(hd) => format!("Hdg   {:5.1} deg", hd),
        None => "Hdg   --".to_string(),
    };
    let lines = [
        format!("Speed {:6.1} m/s", a.speed),
        format!("Path  {:+6.1} deg", a.path),
        format!("Pitch {:+6.1} deg", a.pitch),
        format!("AoA   {:+6.1} deg", a.aoa),
        heading_text,
    ];
    for (i, s) in lines.iter().enumerate() {
        draw_text(
            frame,
            w,
            h,
            font,
            s,
            x0 + 10,
            y0 + 160 + i as i32 * 24,
            22.0,
            [255, 255, 255],
        );
    }
}

pub fn draw_bank_gauge(
    frame: &mut [u8],
    w: usize,
    h: usize,
    font: &Font,
    bank_deg: f32, // degrees, same unit as sprite.bank_angle
    x0: i32,
    y0: i32,
) {
    const PANEL_W: i32 = 260;
    const PANEL_H: i32 = 175;
    const GREEN: [u8; 3] = [80, 255, 120];
    const ORANGE: [u8; 3] = [255, 170, 40];
    const GREY: [u8; 3] = [110, 110, 140];

    fill_rect(frame, w, h, x0, y0, PANEL_W, PANEL_H, [10, 10, 25], 0.6);
    draw_text(
        frame,
        w,
        h,
        font,
        "Bank (view along motion)",
        x0 + 10,
        y0 + 6,
        18.0,
        [255, 255, 255],
    );

    let cx = (x0 + 75) as f32;
    let cy = (y0 + 98) as f32;
    let r = 52.0;

    // dial with ticks every 90 degrees; the top tick is longer and marks "up"
    draw_arc(frame, w, h, cx, cy, r, 0.0, 360.0, GREY);
    for k in 0..4 {
        let a = 90.0 + k as f32 * 90.0;
        let inner = if k == 0 { r - 12.0 } else { r - 6.0 };
        let (x1, y1) = polar(cx, cy, inner, a);
        let (x2, y2) = polar(cx, cy, r, a);
        draw_line(frame, w, h, x1, y1, x2, y2, GREY);
    }

    // screen angle of the lift arrow: bank 0 -> 90 (up), bank +90 -> 0 (right)
    let lift_angle = 90.0 - bank_deg;

    // wings bar, perpendicular to the lift arrow
    let (wx1, wy1) = polar(cx, cy, r * 0.8, lift_angle + 90.0);
    let (wx2, wy2) = polar(cx, cy, r * 0.8, lift_angle - 90.0);
    draw_line(frame, w, h, wx1, wy1, wx2, wy2, ORANGE);

    // lift arrow
    draw_arrow(frame, w, h, cx, cy, r * 0.9, lift_angle, GREEN);

    // numbers
    let shown = (bank_deg + 180.0).rem_euclid(360.0) - 180.0; // wrap to -180..180
    let up_share = bank_deg.to_radians().cos() * 100.0;
    let side_share = bank_deg.to_radians().sin() * 100.0;
    let side_dir = if side_share >= 0.0 { "R" } else { "L" };

    draw_text(
        frame,
        w,
        h,
        font,
        &format!("Bank {:+6.1}", shown),
        x0 + 145,
        y0 + 55,
        20.0,
        [255, 255, 255],
    );
    draw_text(
        frame,
        w,
        h,
        font,
        &format!("Up   {:+4.0}%", up_share),
        x0 + 145,
        y0 + 85,
        20.0,
        [255, 255, 255],
    );
    draw_text(
        frame,
        w,
        h,
        font,
        &format!("Side {:4.0}% {}", side_share.abs(), side_dir),
        x0 + 145,
        y0 + 115,
        20.0,
        [255, 255, 255],
    );
}
