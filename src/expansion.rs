use crate::graph::{Node, SIGNAL_CYCLE};
use crate::world::World;
use std::collections::HashMap;

pub const GRID_SPACING: f64 = 110.0;
pub const MAX_CITY_COLS: i32 = 12;
pub const MAX_CITY_ROWS: i32 = 8;
pub const MAX_CITY_NODES: usize = 96;

impl World {
    pub fn generate(seed: u64, init_cols: usize, init_rows: usize) -> World {
        let mut w = World::empty(seed);
        let cols = init_cols.clamp(3, 8);
        let rows = init_rows.clamp(2, 6);

        w.gc = [0, (cols as i32) - 1, 0, (rows as i32) - 1];

        for r in 0..rows {
            for c in 0..cols {
                let x = c as f64 * GRID_SPACING + w.rng.between(-12.0, 12.0);
                let y = r as f64 * GRID_SPACING + w.rng.between(-12.0, 12.0);
                let node = Node::new(x, y, 0.0);
                w.nodes.push(node);
                let id = w.nodes.len() - 1;
                w.gid.insert((c as i32, r as i32), id);
            }
        }

        let get_id = |c: i32, r: i32, gid: &HashMap<(i32, i32), usize>| *gid.get(&(c, r)).unwrap();
        for r in 0..(rows as i32) {
            for c in 0..(cols as i32) {
                let u = get_id(c, r, &w.gid);
                if c + 1 < (cols as i32) {
                    let v = get_id(c + 1, r, &w.gid);
                    let lanes = if r == 1 || r == 3 { 2 } else { 1 };
                    let vmax = if lanes > 1 { 19.4 } else { 13.9 };
                    w.add_edge(u, v, vmax, lanes, None, false);
                    w.add_edge(v, u, vmax, lanes, None, false);
                }
                if r + 1 < (rows as i32) {
                    let v = get_id(c, r + 1, &w.gid);
                    let lanes = if c == 1 || c == 3 { 2 } else { 1 };
                    let vmax = if lanes > 1 { 19.4 } else { 13.9 };
                    w.add_edge(u, v, vmax, lanes, None, false);
                    w.add_edge(v, u, vmax, lanes, None, false);
                }
            }
        }

        for node in w.nodes.iter_mut() {
            node.offset = w.rng.f() * SIGNAL_CYCLE;
        }

        w.rebuild();

        for i in 0..w.nodes.len() {
            w.spawn_buildings_for_node(i);
        }

        w.populate_citizens(60);

        let (bx0, by0, bx1, by1) = w.bounds();
        let cx = (bx0 + bx1) / 2.0;
        let cy = (by0 + by1) / 2.0;
        let r = (bx1 - bx0).hypot(by1 - by0) / 2.0 + 40.0;
        let active_nodes: Vec<usize> = (0..w.nodes.len()).collect();
        let all_edges: Vec<usize> = (0..w.edges.len()).collect();

        w.record_change(
            0,
            "City Founded".into(),
            format!(
                "Initial settlement built with {} intersections, {} roads, and {} buildings.",
                w.nodes.len(),
                w.edges.len() / 2,
                w.buildings.len()
            ),
            cx,
            cy,
            r,
            all_edges,
            active_nodes,
        );

        w
    }

    pub fn expand_city(&mut self) -> bool {
        let [c0, c1, r0, r1] = self.gc;
        let width = c1 - c0 + 1;
        let height = r1 - r0 + 1;

        if (width >= MAX_CITY_COLS && height >= MAX_CITY_ROWS) || self.nodes.len() >= MAX_CITY_NODES
        {
            return false;
        }

        let mut valid_dirs = Vec::new();
        if width < MAX_CITY_COLS {
            valid_dirs.push(1);
            valid_dirs.push(3);
        }
        if height < MAX_CITY_ROWS {
            valid_dirs.push(2);
            valid_dirs.push(0);
        }

        if valid_dirs.is_empty() {
            return false;
        }

        self.expansions += 1;

        let dir = if width <= height && (valid_dirs.contains(&1) || valid_dirs.contains(&3)) {
            if valid_dirs.contains(&1) && (self.rng.f() > 0.4 || !valid_dirs.contains(&3)) {
                1
            } else {
                3
            }
        } else if valid_dirs.contains(&2) || valid_dirs.contains(&0) {
            if valid_dirs.contains(&2) && (self.rng.f() > 0.4 || !valid_dirs.contains(&0)) {
                2
            } else {
                0
            }
        } else {
            valid_dirs[self.rng.range(valid_dirs.len())]
        };

        let mut new_nodes = vec![];
        let mut new_edges = vec![];
        let target_c_range = c0..=c1;
        let target_r_range = r0..=r1;

        match dir {
            1 => {
                let nc = c1 + 1;
                self.gc[1] = nc;
                for r in target_r_range {
                    let base_y = r as f64 * GRID_SPACING;
                    let x = nc as f64 * GRID_SPACING + self.rng.between(-12.0, 12.0);
                    let y = base_y + self.rng.between(-12.0, 12.0);
                    let node = Node::new(x, y, self.time);
                    self.nodes.push(node);
                    let nid = self.nodes.len() - 1;
                    self.gid.insert((nc, r), nid);
                    new_nodes.push(nid);
                }
                for r in r0..=r1 {
                    let curr = *self.gid.get(&(nc, r)).unwrap();
                    let prev = *self.gid.get(&(nc - 1, r)).unwrap();
                    let e1 = self.add_edge(prev, curr, 13.9, 1, None, false);
                    let e2 = self.add_edge(curr, prev, 13.9, 1, None, false);
                    new_edges.push(e1);
                    new_edges.push(e2);
                    if r > r0 {
                        let up = *self.gid.get(&(nc, r - 1)).unwrap();
                        let e3 = self.add_edge(up, curr, 13.9, 1, None, false);
                        let e4 = self.add_edge(curr, up, 13.9, 1, None, false);
                        new_edges.push(e3);
                        new_edges.push(e4);
                    }
                }
            }
            2 => {
                let nr = r1 + 1;
                self.gc[3] = nr;
                for c in target_c_range {
                    let base_x = c as f64 * GRID_SPACING;
                    let x = base_x + self.rng.between(-12.0, 12.0);
                    let y = nr as f64 * GRID_SPACING + self.rng.between(-12.0, 12.0);
                    let node = Node::new(x, y, self.time);
                    self.nodes.push(node);
                    let nid = self.nodes.len() - 1;
                    self.gid.insert((c, nr), nid);
                    new_nodes.push(nid);
                }
                for c in c0..=c1 {
                    let curr = *self.gid.get(&(c, nr)).unwrap();
                    let above = *self.gid.get(&(c, nr - 1)).unwrap();
                    let e1 = self.add_edge(above, curr, 13.9, 1, None, false);
                    let e2 = self.add_edge(curr, above, 13.9, 1, None, false);
                    new_edges.push(e1);
                    new_edges.push(e2);
                    if c > c0 {
                        let left = *self.gid.get(&(c - 1, nr)).unwrap();
                        let e3 = self.add_edge(left, curr, 13.9, 1, None, false);
                        let e4 = self.add_edge(curr, left, 13.9, 1, None, false);
                        new_edges.push(e3);
                        new_edges.push(e4);
                    }
                }
            }
            0 => {
                let nr = r0 - 1;
                self.gc[2] = nr;
                for c in target_c_range {
                    let base_x = c as f64 * GRID_SPACING;
                    let x = base_x + self.rng.between(-12.0, 12.0);
                    let y = nr as f64 * GRID_SPACING + self.rng.between(-12.0, 12.0);
                    let node = Node::new(x, y, self.time);
                    self.nodes.push(node);
                    let nid = self.nodes.len() - 1;
                    self.gid.insert((c, nr), nid);
                    new_nodes.push(nid);
                }
                for c in c0..=c1 {
                    let curr = *self.gid.get(&(c, nr)).unwrap();
                    let below = *self.gid.get(&(c, nr + 1)).unwrap();
                    let e1 = self.add_edge(below, curr, 13.9, 1, None, false);
                    let e2 = self.add_edge(curr, below, 13.9, 1, None, false);
                    new_edges.push(e1);
                    new_edges.push(e2);
                    if c > c0 {
                        let left = *self.gid.get(&(c - 1, nr)).unwrap();
                        let e3 = self.add_edge(left, curr, 13.9, 1, None, false);
                        let e4 = self.add_edge(curr, left, 13.9, 1, None, false);
                        new_edges.push(e3);
                        new_edges.push(e4);
                    }
                }
            }
            _ => {
                let nc = c0 - 1;
                self.gc[0] = nc;
                for r in target_r_range {
                    let base_y = r as f64 * GRID_SPACING;
                    let x = nc as f64 * GRID_SPACING + self.rng.between(-12.0, 12.0);
                    let y = base_y + self.rng.between(-12.0, 12.0);
                    let node = Node::new(x, y, self.time);
                    self.nodes.push(node);
                    let nid = self.nodes.len() - 1;
                    self.gid.insert((nc, r), nid);
                    new_nodes.push(nid);
                }
                for r in r0..=r1 {
                    let curr = *self.gid.get(&(nc, r)).unwrap();
                    let next = *self.gid.get(&(nc + 1, r)).unwrap();
                    let e1 = self.add_edge(next, curr, 13.9, 1, None, false);
                    let e2 = self.add_edge(curr, next, 13.9, 1, None, false);
                    new_edges.push(e1);
                    new_edges.push(e2);
                    if r > r0 {
                        let up = *self.gid.get(&(nc, r - 1)).unwrap();
                        let e3 = self.add_edge(up, curr, 13.9, 1, None, false);
                        let e4 = self.add_edge(curr, up, 13.9, 1, None, false);
                        new_edges.push(e3);
                        new_edges.push(e4);
                    }
                }
            }
        }

        self.rebuild();

        let prev_bldg_cnt = self.buildings.len();
        for &nid in &new_nodes {
            self.spawn_buildings_for_node(nid);
        }
        let added_bldgs = self.buildings.len() - prev_bldg_cnt;

        let citizens_to_add = (new_nodes.len() * 12).max(24);
        self.populate_citizens(citizens_to_add);

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for &nid in &new_nodes {
            let n = &self.nodes[nid];
            min_x = min_x.min(n.x);
            min_y = min_y.min(n.y);
            max_x = max_x.max(n.x);
            max_y = max_y.max(n.y);
        }
        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;
        let radius = (max_x - min_x).hypot(max_y - min_y) / 2.0 + 70.0;

        let dir_str = match dir {
            0 => "North",
            1 => "East",
            2 => "South",
            _ => "West",
        };

        self.record_change(
            0,
            format!("City Expansion #{} ({} District)", self.expansions, dir_str),
            format!(
                "Added {} new intersections, {} road segments, and {} buildings. Total citizens: {}.",
                new_nodes.len(),
                new_edges.len() / 2,
                added_bldgs,
                self.citizens.len()
            ),
            cx,
            cy,
            radius,
            new_edges,
            new_nodes,
        );

        if self.is_max_city() {
            let (bx0, by0, bx1, by1) = self.bounds();
            self.record_change(
                0,
                "Maximum City Size Reached".into(),
                format!(
                    "The metropolis has reached its maximum territorial capacity ({} nodes, {} buildings, {} citizens). Future planner interventions will focus on capacity, roundabouts, and bypasses.",
                    self.nodes.len(),
                    self.buildings.len(),
                    self.citizens.len()
                ),
                (bx0 + bx1) / 2.0,
                (by0 + by1) / 2.0,
                (bx1 - bx0).hypot(by1 - by0) / 2.0 + 40.0,
                vec![],
                vec![],
            );
        }

        true
    }

    pub fn is_max_city(&self) -> bool {
        let width = self.gc[1] - self.gc[0] + 1;
        let height = self.gc[3] - self.gc[2] + 1;
        (width >= MAX_CITY_COLS && height >= MAX_CITY_ROWS) || self.nodes.len() >= MAX_CITY_NODES
    }
}
