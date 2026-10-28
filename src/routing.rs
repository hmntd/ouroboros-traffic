use crate::vehicle::VMAX_GLOBAL;
use crate::world::World;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

impl World {
    pub fn astar(&self, s: usize, t: usize) -> Option<Vec<usize>> {
        let n = self.nodes.len();
        let mut g = vec![f64::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();
        let h = |i: usize| {
            ((self.nodes[i].x - self.nodes[t].x).hypot(self.nodes[i].y - self.nodes[t].y))
                / VMAX_GLOBAL
        };
        g[s] = 0.0;
        heap.push(Reverse(((h(s) * 1000.0) as u64, s)));
        while let Some(Reverse((_, u))) = heap.pop() {
            if u == t {
                break;
            }
            for &ei in &self.out_adj[u] {
                let e = &self.edges[ei];
                let spd = e.tt_speed.min(e.vmax).max(1.0);
                let mut cost = e.length / spd;
                if self.nodes[e.to].signal {
                    cost += 4.0;
                }
                let ng = g[u] + cost;
                if ng < g[e.to] {
                    g[e.to] = ng;
                    prev[e.to] = ei;
                    heap.push(Reverse((((ng + h(e.to)) * 1000.0) as u64, e.to)));
                }
            }
        }
        if !g[t].is_finite() {
            return None;
        }
        let mut path = vec![];
        let mut cur = t;
        while cur != s {
            let ei = prev[cur];
            path.push(ei);
            cur = self.edges[ei].from;
        }
        path.reverse();
        Some(path)
    }

    pub fn ped_path(&self, s: usize, t: usize) -> Option<Vec<usize>> {
        let n = self.nodes.len();
        let mut g = vec![f64::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();
        g[s] = 0.0;
        heap.push(Reverse((0u64, s)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if u == t {
                break;
            }
            if (d as f64) / 100.0 > g[u] + 1e-6 {
                continue;
            }
            for &(v, len) in &self.ped_adj[u] {
                let ng = g[u] + len;
                if ng < g[v] {
                    g[v] = ng;
                    prev[v] = u;
                    heap.push(Reverse(((ng * 100.0) as u64, v)));
                }
            }
        }
        if !g[t].is_finite() {
            return None;
        }
        let mut path = vec![t];
        let mut cur = t;
        while cur != s {
            cur = prev[cur];
            path.push(cur);
        }
        path.reverse();
        Some(path)
    }

    pub fn reroute_vehicle(&mut self, i: usize) {
        let (cur, dest) = {
            let v = &self.vehicles[i];
            (v.edge, v.dest)
        };
        let from = self.edges[cur].to;
        if from == dest {
            let v = &mut self.vehicles[i];
            v.path = vec![cur];
            v.pi = 0;
            return;
        }
        if let Some(p) = self.astar(from, dest) {
            let v = &mut self.vehicles[i];
            let mut np = vec![cur];
            np.extend(p);
            v.path = np;
            v.pi = 0;
        }
    }

    pub fn reroute_all(&mut self) {
        for i in 0..self.vehicles.len() {
            self.reroute_vehicle(i);
        }
    }
}
