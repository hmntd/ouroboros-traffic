use crate::graph::{MAX_LANES, Node, RING_R};
use crate::world::World;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Mod {
    Widen(usize),
    Roundabout(usize),
    Bypass(usize, usize, usize),
}

impl World {
    pub fn worst_nodes(&self, k: usize) -> Vec<usize> {
        let mut ids: Vec<usize> = (0..self.nodes.len())
            .filter(|&i| self.nodes[i].active && !self.in_adj[i].is_empty())
            .collect();
        ids.sort_by(|&a, &b| self.nodes[b].jam.partial_cmp(&self.nodes[a].jam).unwrap());
        ids.truncate(k);
        ids
    }

    pub fn connected(&self, a: usize, b: usize) -> bool {
        self.out_adj[a].iter().any(|&e| self.edges[e].to == b)
            || self.out_adj[b].iter().any(|&e| self.edges[e].to == a)
    }

    pub fn can_widen(&self, n: usize) -> bool {
        self.in_adj[n]
            .iter()
            .chain(self.out_adj[n].iter())
            .any(|&e| self.edges[e].lanes < MAX_LANES && !self.edges[e].ring)
    }

    fn neighbours(&self, n: usize) -> Vec<usize> {
        let mut nb = vec![];
        for &e in &self.in_adj[n] {
            let s = self.edges[e].from;
            if !nb.contains(&s) {
                nb.push(s);
            }
        }
        for &e in &self.out_adj[n] {
            let t = self.edges[e].to;
            if !nb.contains(&t) {
                nb.push(t);
            }
        }
        nb
    }

    pub fn can_roundabout(&self, n: usize) -> bool {
        let node = &self.nodes[n];
        node.active && !node.ring && !node.island && self.neighbours(n).len() >= 3
    }

    pub fn bypass_candidate(&self, j: usize) -> Option<Mod> {
        let nb: Vec<usize> = self
            .neighbours(j)
            .into_iter()
            .filter(|&x| self.nodes[x].active && !self.nodes[x].ring && self.nodes[x].jam < 0.4)
            .collect();
        let mut best: Option<(f64, usize, usize)> = None;
        for (ia, &a) in nb.iter().enumerate() {
            for &b in nb.iter().skip(ia + 1) {
                if self.connected(a, b) {
                    continue;
                }
                let d =
                    (self.nodes[a].x - self.nodes[b].x).hypot(self.nodes[a].y - self.nodes[b].y);
                if d > 320.0 {
                    continue;
                }
                let score = self.nodes[a].jam + self.nodes[b].jam + d / 1000.0;
                if best.is_none_or(|bb| score < bb.0) {
                    best = Some((score, a, b));
                }
            }
        }
        best.map(|(_, a, b)| Mod::Bypass(a, b, j))
    }

    pub fn describe(&self, m: &Mod) -> String {
        match m {
            Mod::Widen(n) => format!("Widen roads at node {}", n),
            Mod::Roundabout(n) => format!("Roundabout at node {}", n),
            Mod::Bypass(a, b, _) => format!("Bypass {} - {}", a, b),
        }
    }

    pub fn apply_mod(&mut self, m: &Mod) {
        match *m {
            Mod::Widen(n) => {
                let mut ids: Vec<usize> = self.in_adj[n]
                    .iter()
                    .chain(self.out_adj[n].iter())
                    .copied()
                    .collect();
                for &e in &self.in_adj[n] {
                    for &u in &self.in_adj[self.edges[e].from] {
                        if self.edges[u].jam > 0.15 {
                            ids.push(u);
                        }
                    }
                }
                let mut done = vec![];
                for e in ids {
                    let ed = &mut self.edges[e];
                    if ed.lanes < MAX_LANES && !ed.ring {
                        ed.lanes += 1;
                        ed.widened = self.time;
                        done.push(e);
                    }
                }
                self.mods[0] += 1;
                let (x, y) = (self.nodes[n].x, self.nodes[n].y);
                self.record_change(
                    1,
                    format!("Widened roads at node {}", n),
                    format!(
                        "+1 lane on {} road segments around the junction.",
                        done.len()
                    ),
                    x,
                    y,
                    130.0,
                    done,
                    vec![],
                );
            }
            Mod::Roundabout(c) => self.make_roundabout(c),
            Mod::Bypass(a, b, j) => {
                let (ax, ay, bx, by) = (
                    self.nodes[a].x,
                    self.nodes[a].y,
                    self.nodes[b].x,
                    self.nodes[b].y,
                );
                let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                let (dx, dy) = (bx - ax, by - ay);
                let l = dx.hypot(dy).max(1.0);
                let (mut nx, mut ny) = (-dy / l, dx / l);
                let (jx, jy) = (self.nodes[j].x, self.nodes[j].y);
                if (mx - jx) * nx + (my - jy) * ny < 0.0 {
                    nx = -nx;
                    ny = -ny;
                }
                let bend = l * 0.18;
                let ctrl = Some((mx + nx * bend, my + ny * bend));
                let e1 = self.add_edge(a, b, 16.7, 1, ctrl, false);
                let e2 = self.add_edge(b, a, 16.7, 1, ctrl, false);
                self.mods[2] += 1;
                self.rebuild();
                self.record_change(
                    3,
                    format!("Bypass {} - {}", a, b),
                    format!(
                        "New curved two-way road ({:.0} m) skirting the jam at node {}.",
                        l, j
                    ),
                    mx,
                    my,
                    l / 2.0 + 60.0,
                    vec![e1, e2],
                    vec![],
                );
            }
        }
        self.reroute_all();
    }

    fn make_roundabout(&mut self, c: usize) {
        let (cx, cy) = (self.nodes[c].x, self.nodes[c].y);
        let nbrs = self.neighbours(c);
        let mut items: Vec<(f64, usize)> = nbrs
            .iter()
            .map(|&nb| ((self.nodes[nb].y - cy).atan2(self.nodes[nb].x - cx), nb))
            .collect();
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let n = items.len();
        let mut ring_ids = vec![];
        let mut map = HashMap::new();
        for &(ang, nb) in &items {
            let mut node = Node::new(cx + RING_R * ang.cos(), cy + RING_R * ang.sin(), self.time);
            node.ring = true;
            self.nodes.push(node);
            let id = self.nodes.len() - 1;
            ring_ids.push(id);
            map.insert(nb, id);
        }
        let ins = self.in_adj[c].clone();
        let outs = self.out_adj[c].clone();
        let mut touched = vec![];
        for e in ins {
            self.edges[e].to = map[&self.edges[e].from];
            self.edges[e].born = self.time;
            touched.push(e);
        }
        for e in outs {
            self.edges[e].from = map[&self.edges[e].to];
            self.edges[e].born = self.time;
            touched.push(e);
        }
        for k in 0..n {
            let kp = (k + n - 1) % n;
            let (a_k, a_p) = (items[k].0, items[kp].0);
            let mut span = (a_k - a_p).rem_euclid(std::f64::consts::TAU);
            if span < 0.05 {
                span = std::f64::consts::TAU;
            }
            let mid = a_k - span / 2.0;
            let rr = RING_R / (span / 2.0).cos().max(0.35);
            let ctrl = Some((cx + rr * mid.cos(), cy + rr * mid.sin()));
            let id = self.add_edge(ring_ids[k], ring_ids[kp], 8.3, 1, ctrl, true);
            touched.push(id);
        }
        self.nodes[c].active = false;
        self.nodes[c].island = true;
        self.nodes[c].jam = 0.0;
        for e in 0..self.edges.len() {
            if !self.edges[e].ring {
                let nodes = &self.nodes;
                let mut ed = self.edges[e].clone();
                ed.recompute(nodes);
                self.edges[e] = ed;
            }
        }
        self.rebuild();
        for b in 0..self.buildings.len() {
            if self.buildings[b].node == c {
                let (bx, by) = (self.buildings[b].x, self.buildings[b].y);
                let best = ring_ids
                    .iter()
                    .copied()
                    .min_by(|&p, &q| {
                        let dp = (self.nodes[p].x - bx).hypot(self.nodes[p].y - by);
                        let dq = (self.nodes[q].x - bx).hypot(self.nodes[q].y - by);
                        dp.partial_cmp(&dq).unwrap()
                    })
                    .unwrap();
                self.buildings[b].node = best;
            }
        }
        for i in 0..self.vehicles.len() {
            let (len, lanes) = {
                let e = &self.edges[self.vehicles[i].edge];
                (e.length, e.lanes)
            };
            let db = self.vehicles[i].dest_b as usize;
            let dn = self.buildings[db].node;
            let v = &mut self.vehicles[i];
            v.dest = dn;
            v.pos = v.pos.min(len - 0.01);
            v.lane = v.lane.min(lanes - 1);
        }
        self.reroute_peds_through(c);
        self.mods[1] += 1;
        self.record_change(
            2,
            format!("Roundabout at node {}", c),
            format!(
                "{}-way signalised junction replaced by a one-way ring of {} nodes.",
                n, n
            ),
            cx,
            cy,
            110.0,
            touched,
            vec![],
        );
    }
}
