use crate::world::World;

#[derive(Clone, Debug)]
pub struct Building {
    pub id: u32,
    pub node: usize,
    pub x: f64,
    pub y: f64,
    pub kind: u8,
    pub parking_cap: usize,
    pub cars_parked: usize,
    pub color_idx: u8,
    pub born: f64,
}

impl World {
    pub fn spawn_buildings_for_node(&mut self, node_idx: usize) {
        let (nx, ny) = (self.nodes[node_idx].x, self.nodes[node_idx].y);
        let quads = [(26.0, 26.0), (-26.0, 26.0), (-26.0, -26.0), (26.0, -26.0)];

        let num_bldgs = if self.rng.f() > 0.35 { 2 } else { 1 };
        let start_q = self.rng.range(4);

        for step in 0..num_bldgs {
            let q = (start_q + step) % 4;
            if (self.nodes[node_idx].bmask & (1 << q)) != 0 {
                continue;
            }
            self.nodes[node_idx].bmask |= 1 << q;

            let bx = nx + quads[q].0 + self.rng.between(-4.0, 4.0);
            let by = ny + quads[q].1 + self.rng.between(-4.0, 4.0);

            let roll = self.rng.f();
            let kind = if roll < 0.50 {
                0
            } else if roll < 0.85 {
                1
            } else {
                2
            };

            let cap = if kind == 0 { 12 } else { 18 };
            let parked = (cap as f64 * self.rng.between(0.4, 0.8)) as usize;
            let color_idx = (self.rng.range(8)) as u8;

            let b_id = self.buildings.len() as u32;
            self.buildings.push(Building {
                id: b_id,
                node: node_idx,
                x: bx,
                y: by,
                kind,
                parking_cap: cap,
                cars_parked: parked,
                color_idx,
                born: self.time,
            });
        }
    }
}
