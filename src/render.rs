use crate::city::{CROSSING_DIST, PPhase};
use crate::sim::{CAR_LEN, Edge, LANE_W, RING_R, World};
use web_sys::CanvasRenderingContext2d;

pub struct Camera {
    pub cx: f64,
    pub cy: f64,
    pub bw: f64,
    pub bh: f64,
    pub zoom: f64,
    pub px: f64,
    pub py: f64,
}

impl Camera {
    pub fn new(world: &World) -> Camera {
        let b = world.bounds();
        Camera {
            cx: (b.0 + b.2) / 2.0,
            cy: (b.1 + b.3) / 2.0,
            bw: (b.2 - b.0 + 120.0).max(180.0),
            bh: (b.3 - b.1 + 120.0).max(180.0),
            zoom: 1.0,
            px: 0.0,
            py: 0.0,
        }
    }

    pub fn scale(&self, w: f64, h: f64) -> f64 {
        (w / self.bw).min(h / self.bh) * self.zoom
    }

    pub fn center_on(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.cx = target_x;
        self.cy = target_y;
        self.px = 0.0;
        self.py = 0.0;
        self.zoom = zoom.clamp(0.4, 6.0);
    }
}

#[derive(Clone, Copy)]
pub struct ViewOpts {
    pub show_nodes: bool,
    pub show_paths: bool,
    pub show_heat: bool,
}

const CAR_PALETTE: [&str; 8] = [
    "#f2f2f2", "#8ecae6", "#ffb703", "#fb8500", "#b5e48c", "#cdb4db", "#ff8fa3", "#90e0ef",
];

const BLDG_PALETTES: [[&str; 2]; 3] = [
    ["#233d4d", "#fe7f2d"],
    ["#1e293b", "#38bdf8"],
    ["#362842", "#a855f7"],
];

fn heat(j: f64) -> String {
    let x = (j / 0.55).clamp(0.0, 1.0);
    let (r, g, b) = if x < 0.5 {
        let k = x * 2.0;
        (
            46.0 + (241.0 - 46.0) * k,
            204.0 + (196.0 - 204.0) * k,
            113.0 + (15.0 - 113.0) * k,
        )
    } else {
        let k = (x - 0.5) * 2.0;
        (
            241.0 + (231.0 - 241.0) * k,
            196.0 + (62.0 - 196.0) * k,
            15.0 + (60.0 - 15.0) * k,
        )
    };
    format!("rgb({},{},{})", r as u8, g as u8, b as u8)
}

fn trace(
    ctx: &CanvasRenderingContext2d,
    e: &Edge,
    world: &World,
    off: f64,
    t1: f64,
    tf: &impl Fn(f64, f64) -> (f64, f64),
) {
    let segs = if e.ctrl.is_some() { 12 } else { 1 };
    ctx.begin_path();
    for i in 0..=segs {
        let t = t1 * i as f64 / segs as f64;
        let (x, y, _) = e.point(&world.nodes, t, off);
        let (sx, sy) = tf(x, y);
        if i == 0 {
            ctx.move_to(sx, sy);
        } else {
            ctx.line_to(sx, sy);
        }
    }
}

pub fn draw(
    ctx: &CanvasRenderingContext2d,
    w: f64,
    h: f64,
    world: &World,
    cam: &Camera,
    opts: ViewOpts,
) {
    let s = cam.scale(w, h);
    let tf = |x: f64, y: f64| {
        (
            (x - cam.cx) * s + w / 2.0 + cam.px,
            (y - cam.cy) * s + h / 2.0 + cam.py,
        )
    };

    let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    ctx.set_global_alpha(1.0);
    ctx.set_fill_style_str("#12141a");
    ctx.fill_rect(0.0, 0.0, w, h);
    ctx.set_line_cap("butt");
    ctx.set_line_join("round");

    ctx.set_stroke_style_str("rgba(80, 90, 115, 0.35)");
    ctx.set_line_width((1.5 * s).max(1.0));
    for b in &world.buildings {
        let (bx, by) = tf(b.x, b.y);
        let n = &world.nodes[b.node];
        let (nx, ny) = tf(n.x, n.y);
        ctx.begin_path();
        ctx.move_to(bx, by);
        ctx.line_to(nx, ny);
        ctx.stroke();
    }

    for e in &world.edges {
        let age = world.time - e.born;
        let prog = (age / 1.5).clamp(0.0, 1.0);
        if prog <= 0.0 {
            continue;
        }
        let off = e.center_offset();
        let width = (e.lanes as f64 * LANE_W).max(LANE_W);

        trace(ctx, e, world, off, prog, &tf);
        ctx.set_stroke_style_str("#252934");
        ctx.set_line_width(width * s);
        ctx.stroke();

        if opts.show_heat {
            trace(ctx, e, world, off, prog, &tf);
            ctx.set_stroke_style_str(&heat(e.jam));
            ctx.set_global_alpha(0.55);
            ctx.set_line_width((width * 0.35).max(1.0) * s);
            ctx.stroke();
            ctx.set_global_alpha(1.0);
        }

        let glow_age = age.min(world.time - e.widened);
        if glow_age < 5.0 {
            trace(ctx, e, world, off, prog, &tf);
            ctx.set_stroke_style_str("#38e8ff");
            ctx.set_global_alpha(0.6 * (1.0 - glow_age / 5.0));
            ctx.set_line_width((width + 3.0) * s);
            ctx.stroke();
            ctx.set_global_alpha(1.0);
        }
    }

    for (ei, e) in world.edges.iter().enumerate() {
        if e.ring || e.length < 25.0 {
            continue;
        }
        let cross_t = (1.0 - CROSSING_DIST / e.length).clamp(0.1, 0.9);
        let (cx, cy, ang) = e.point(&world.nodes, cross_t, e.center_offset());
        let (scx, scy) = tf(cx, cy);

        let active = world.cross_cnt[ei] > 0;
        let width = (e.lanes as f64 * LANE_W).max(LANE_W) * s;
        let stripe_len = (2.8 * s).max(2.5);

        let c_ang = ang.cos();
        let s_ang = ang.sin();
        let _ = ctx.set_transform(c_ang, s_ang, -s_ang, c_ang, scx, scy);

        if active {
            ctx.set_fill_style_str("rgba(0, 245, 212, 0.28)");
            ctx.fill_rect(-stripe_len * 1.5, -width / 2.0, stripe_len * 3.0, width);
        }

        ctx.set_fill_style_str(if active {
            "#00f5d4"
        } else {
            "rgba(235, 240, 255, 0.65)"
        });
        let stripes = (e.lanes * 3).max(3);
        let step = width / (stripes as f64 + 1.0);
        for k in 1..=stripes {
            let y_off = -width / 2.0 + (k as f64) * step;
            ctx.fill_rect(-stripe_len / 2.0, y_off - 1.2, stripe_len, 2.4);
        }
    }
    let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    let bw = (22.0 * s).clamp(8.0, 48.0);
    let bh = (18.0 * s).clamp(7.0, 40.0);
    for b in &world.buildings {
        let (bx, by) = tf(b.x, b.y);
        let p_idx = (b.kind as usize).min(2);
        let [body_col, roof_col] = BLDG_PALETTES[p_idx];

        ctx.set_fill_style_str("rgba(0, 0, 0, 0.35)");
        ctx.fill_rect(bx - bw / 2.0 + 2.0, by - bh / 2.0 + 3.0, bw, bh);

        ctx.set_fill_style_str(body_col);
        ctx.fill_rect(bx - bw / 2.0, by - bh / 2.0, bw, bh);

        ctx.set_fill_style_str(roof_col);
        ctx.fill_rect(bx - bw / 2.0, by - bh / 2.0, bw, (bh * 0.25).max(2.0));

        let pw = (16.0 * s).clamp(6.0, 32.0);
        let ph = (9.0 * s).clamp(4.0, 18.0);
        let px = bx + bw / 2.0 + 2.0;
        let py = by - ph / 2.0;

        ctx.set_fill_style_str("#1a1d26");
        ctx.fill_rect(px, py, pw, ph);
        ctx.set_stroke_style_str("rgba(255, 255, 255, 0.12)");
        ctx.set_line_width(1.0);
        ctx.stroke_rect(px, py, pw, ph);

        let slots = b.cars_parked.min(b.parking_cap);
        let car_w = (pw / 4.0).max(3.0);
        let car_h = (ph / 2.2).max(2.0);
        for k in 0..slots.min(6) {
            let col_k = k % 3;
            let row_k = k / 3;
            let cx = px + (col_k as f64 + 0.5) * (pw / 3.0);
            let cy = py + (row_k as f64 + 0.5) * (ph / 2.0);
            ctx.set_fill_style_str(CAR_PALETTE[(k * 3 + b.color_idx as usize) % 8]);
            ctx.fill_rect(cx - car_w / 2.5, cy - car_h / 2.5, car_w * 0.8, car_h * 0.8);
        }
    }

    for (i, n) in world.nodes.iter().enumerate() {
        let (sx, sy) = tf(n.x, n.y);
        if n.island {
            ctx.set_fill_style_str("#235338");
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, (RING_R - LANE_W) * s, 0.0, std::f64::consts::TAU);
            ctx.fill();
            continue;
        }
        if !n.active {
            continue;
        }
        if !n.ring {
            ctx.set_fill_style_str("#252934");
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, 5.0 * s, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
        if opts.show_nodes {
            ctx.set_fill_style_str(if n.ring { "#ffd166" } else { "#7aa2ff" });
            ctx.begin_path();
            let _ = ctx.arc(
                sx,
                sy,
                if n.ring { 1.5_f64 } else { 3.0_f64 },
                0.0,
                std::f64::consts::TAU,
            );
            ctx.fill();
            if !n.ring && s > 0.8 {
                ctx.set_fill_style_str("#9aa5c0");
                ctx.set_font("10px sans-serif");
                let _ = ctx.fill_text(&format!("{}", i), sx + 6.0, sy - 6.0);
            }
        }
    }

    for e in &world.edges {
        let node = &world.nodes[e.to];
        if node.signal && world.time - e.born > 1.5 {
            let p = (world.time + node.offset).rem_euclid(28.0);
            let green = if e.horizontal {
                p < 12.0
            } else {
                (14.0..26.0).contains(&p)
            };
            let (x, y, _) = e.point(&world.nodes, 1.0 - 4.0 / e.length, e.center_offset());
            let (sx, sy) = tf(x, y);
            ctx.set_fill_style_str(if green { "#3ddc84" } else { "#ff4d4d" });
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, (1.2 * s).max(1.2), 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
    }

    if opts.show_paths {
        ctx.set_stroke_style_str("#38e8ff");
        ctx.set_global_alpha(0.35);
        ctx.set_line_width(1.5);
        let step = (world.vehicles.len() / 40).max(1);
        for v in world.vehicles.iter().step_by(step) {
            ctx.begin_path();
            let (x, y, _) = world.vehicle_pose(v);
            let (sx, sy) = tf(x, y);
            ctx.move_to(sx, sy);
            for &ei in &v.path[v.pi..] {
                let n = &world.nodes[world.edges[ei].to];
                let (sx, sy) = tf(n.x, n.y);
                ctx.line_to(sx, sy);
            }
            ctx.stroke();
            let d = &world.nodes[v.dest];
            let (dx, dy) = tf(d.x, d.y);
            ctx.begin_path();
            let _ = ctx.arc(dx, dy, 3.0, 0.0, std::f64::consts::TAU);
            ctx.stroke();
        }
        ctx.set_global_alpha(1.0);
    }

    let ped_r = (2.2 * s).clamp(2.0, 5.0);
    for p in &world.peds {
        let (sx, sy) = tf(p.x, p.y);
        ctx.begin_path();
        let _ = ctx.arc(sx, sy, ped_r, 0.0, std::f64::consts::TAU);
        if p.phase == PPhase::Cross {
            ctx.set_fill_style_str("#00f5d4");
        } else {
            ctx.set_fill_style_str("#ffd166");
        }
        ctx.fill();
    }

    let vl = (CAR_LEN * 1.25).max(5.5 / s);
    let vw = vl * 0.43;
    for v in &world.vehicles {
        let (x, y, ang) = world.vehicle_pose(v);
        let (sx, sy) = tf(x, y);
        let (c, sn) = (ang.cos() * s, ang.sin() * s);
        let _ = ctx.set_transform(c, sn, -sn, c, sx, sy);
        ctx.set_fill_style_str(CAR_PALETTE[(v.id % 8) as usize]);
        ctx.fill_rect(-vl / 2.0, -vw / 2.0, vl, vw);
        if v.a < -1.2 || v.v < 0.3 {
            ctx.set_fill_style_str("#ff2d2d");
            ctx.fill_rect(-vl / 2.0, -vw / 2.0, 0.9, vw);
        }
    }
    let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    if let Some((fx, fy, fr, start_t)) = world.focus {
        let age = world.time - start_t;
        if age < 5.0 {
            let (sx, sy) = tf(fx, fy);
            let rad = fr * s;
            let alpha = (1.0 - age / 5.0).clamp(0.0, 1.0);

            let wave = (age * 1.8).fract();
            let wave_rad = rad * (0.8 + 0.4 * wave);
            ctx.set_stroke_style_str("#38e8ff");
            ctx.set_global_alpha(0.7 * alpha * (1.0 - wave));
            ctx.set_line_width(2.5);
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, wave_rad, 0.0, std::f64::consts::TAU);
            ctx.stroke();

            ctx.set_global_alpha(0.85 * alpha);
            ctx.set_line_width(2.0);
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, rad, 0.0, std::f64::consts::TAU);
            ctx.stroke();

            ctx.set_fill_style_str("#38e8ff");
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, 4.0, 0.0, std::f64::consts::TAU);
            ctx.fill();
            ctx.set_global_alpha(1.0);
        }
    }
}
