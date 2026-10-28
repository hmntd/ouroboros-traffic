pub const LANE_W: f64 = 3.2;
pub const MAX_LANES: u8 = 3;
pub const RING_R: f64 = 16.0;
pub const JAM_EDGE: f64 = 0.30;
pub const SIGNAL_CYCLE: f64 = 28.0;

#[derive(Clone)]
pub struct Node {
    pub x: f64,
    pub y: f64,
    pub active: bool,
    pub signal: bool,
    pub offset: f64,
    pub ring: bool,
    pub island: bool,
    pub jam: f64,
    pub born: f64,
    pub bmask: u8,
}

impl Node {
    pub fn new(x: f64, y: f64, time: f64) -> Node {
        Node {
            x,
            y,
            active: true,
            signal: false,
            offset: 0.0,
            ring: false,
            island: false,
            jam: 0.0,
            born: time,
            bmask: 0,
        }
    }
}

#[derive(Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub length: f64,
    pub vmax: f64,
    pub lanes: u8,
    pub ctrl: Option<(f64, f64)>,
    pub ring: bool,
    pub born: f64,
    pub jam: f64,
    pub tt_speed: f64,
    pub avg_speed: f64,
    pub count: usize,
    pub horizontal: bool,
    pub widened: f64,
    pub rev: Option<usize>,
}

impl Edge {
    pub fn base_offset(&self, lane: u8) -> f64 {
        if self.ring {
            (lane as f64 - (self.lanes as f64 - 1.0) / 2.0) * LANE_W
        } else {
            (lane as f64 + 0.5) * LANE_W
        }
    }

    pub fn center_offset(&self) -> f64 {
        if self.ring {
            0.0
        } else {
            self.lanes as f64 * LANE_W / 2.0
        }
    }

    pub fn point(&self, nodes: &[Node], t: f64, off: f64) -> (f64, f64, f64) {
        let a = &nodes[self.from];
        let b = &nodes[self.to];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let l = dx.hypot(dy).max(1e-6);
        let nx = -dy / l * off;
        let ny = dx / l * off;
        let (x0, y0, x1, y1) = (a.x + nx, a.y + ny, b.x + nx, b.y + ny);
        match self.ctrl {
            None => (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, dy.atan2(dx)),
            Some((cx, cy)) => {
                let (cx, cy) = (cx + nx, cy + ny);
                let u = 1.0 - t;
                let x = u * u * x0 + 2.0 * u * t * cx + t * t * x1;
                let y = u * u * y0 + 2.0 * u * t * cy + t * t * y1;
                let tx = 2.0 * u * (cx - x0) + 2.0 * t * (x1 - cx);
                let ty = 2.0 * u * (cy - y0) + 2.0 * t * (y1 - cy);
                (x, y, ty.atan2(tx))
            }
        }
    }

    pub fn recompute(&mut self, nodes: &[Node]) {
        let a = &nodes[self.from];
        let b = &nodes[self.to];
        self.horizontal = (b.x - a.x).abs() >= (b.y - a.y).abs();
        if self.ctrl.is_none() {
            self.length = (b.x - a.x).hypot(b.y - a.y).max(1.0);
        } else {
            let mut len = 0.0;
            let mut prev = self.point(nodes, 0.0, 0.0);
            for i in 1..=16 {
                let p = self.point(nodes, i as f64 / 16.0, 0.0);
                len += (p.0 - prev.0).hypot(p.1 - prev.1);
                prev = p;
            }
            self.length = len.max(1.0);
        }
    }
}

pub fn is_red(time: f64, node: &Node, edge: &Edge) -> bool {
    let p = (time + node.offset).rem_euclid(SIGNAL_CYCLE);
    let green = if edge.horizontal {
        p < 12.0
    } else {
        (14.0..26.0).contains(&p)
    };
    !green
}
