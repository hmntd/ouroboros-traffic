use crate::building::Building;
use crate::change::Change;
use crate::citizen::{Citizen, CitizenState};
use crate::graph::{Edge, JAM_EDGE, Node, is_red};
use crate::pedestrian::{PPhase, Ped};
use crate::rng::Rng;
use crate::vehicle::{A_MAX, B_COMF, B_STOP, CAR_LEN, MIN_GAP, S0, T_HEAD, Vehicle};
use std::collections::HashMap;

#[derive(Clone)]
pub struct World {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub out_adj: Vec<Vec<usize>>,
    pub in_adj: Vec<Vec<usize>>,
    pub ped_adj: Vec<Vec<(usize, f64)>>,
    pub vehicles: Vec<Vehicle>,
    pub buildings: Vec<Building>,
    pub citizens: Vec<Citizen>,
    pub peds: Vec<Ped>,
    pub changes: Vec<Change>,
    pub gc: [i32; 4],
    pub gid: HashMap<(i32, i32), usize>,
    pub time: f64,
    pub rng: Rng,
    pub trip_scale: f64,
    pub dwell_scale: f64,
    pub spawn_rate: f64,
    pub spawn_scale: f64,
    pub spawn_acc: f64,
    pub ped_acc: f64,
    pub spawned: u64,
    pub removed: u64,
    pub focus: Option<(f64, f64, f64, f64)>,
    pub next_id: u32,
    pub step_count: u64,
    pub arrived: u64,
    pub blocked: u64,
    pub teleported: u64,
    pub trips_car: u64,
    pub trips_walk: u64,
    pub flow_per_min: f64,
    pub arr_window: u64,
    pub window_t: f64,
    pub avg_speed: f64,
    pub avg_speed_ratio: f64,
    pub jammed_edges: usize,
    pub max_node_jam: f64,
    pub mods: [u32; 3],
    pub expansions: u32,
    pub hist: Vec<[f32; 3]>,
    pub hist_t: f64,
    pub edge_veh: Vec<Vec<u32>>,
    pub cross_cnt: Vec<u16>,
    pub pending_arrivals: Vec<(u32, u32)>,
}

impl World {
    pub fn empty(seed: u64) -> World {
        World {
            nodes: vec![],
            edges: vec![],
            out_adj: vec![],
            in_adj: vec![],
            ped_adj: vec![],
            vehicles: vec![],
            buildings: vec![],
            citizens: vec![],
            peds: vec![],
            changes: vec![],
            gc: [0, -1, 0, -1],
            gid: HashMap::new(),
            time: 0.0,
            rng: Rng::new(seed),
            trip_scale: 1.0,
            dwell_scale: 1.0,
            spawn_rate: 2.0,
            spawn_scale: 1.0,
            spawn_acc: 0.0,
            ped_acc: 0.0,
            spawned: 0,
            removed: 0,
            focus: None,
            next_id: 0,
            step_count: 0,
            arrived: 0,
            blocked: 0,
            teleported: 0,
            trips_car: 0,
            trips_walk: 0,
            flow_per_min: 0.0,
            arr_window: 0,
            window_t: 0.0,
            avg_speed: 0.0,
            avg_speed_ratio: 1.0,
            jammed_edges: 0,
            max_node_jam: 0.0,
            mods: [0; 3],
            expansions: 0,
            hist: vec![],
            hist_t: 0.0,
            edge_veh: vec![],
            cross_cnt: vec![],
            pending_arrivals: vec![],
        }
    }

    pub fn add_edge(
        &mut self,
        from: usize,
        to: usize,
        vmax: f64,
        lanes: u8,
        ctrl: Option<(f64, f64)>,
        ring: bool,
    ) -> usize {
        let mut e = Edge {
            from,
            to,
            length: 1.0,
            vmax,
            lanes,
            ctrl,
            ring,
            born: self.time,
            jam: 0.0,
            tt_speed: vmax,
            avg_speed: vmax,
            count: 0,
            horizontal: true,
            widened: -100.0,
            rev: None,
        };
        e.recompute(&self.nodes);
        self.edges.push(e);
        self.edges.len() - 1
    }

    pub fn rebuild(&mut self) {
        let n = self.nodes.len();
        self.out_adj = vec![vec![]; n];
        self.in_adj = vec![vec![]; n];
        self.ped_adj = vec![vec![]; n];
        for (i, e) in self.edges.iter().enumerate() {
            self.out_adj[e.from].push(i);
            self.in_adj[e.to].push(i);
            self.ped_adj[e.from].push((e.to, e.length));
            self.ped_adj[e.to].push((e.from, e.length));
        }
        for i in 0..self.edges.len() {
            let (f, t) = (self.edges[i].from, self.edges[i].to);
            self.edges[i].rev = self.out_adj[t]
                .iter()
                .copied()
                .find(|&j| self.edges[j].to == f);
        }
        for i in 0..n {
            let node = &mut self.nodes[i];
            node.signal = node.active && !node.ring && self.in_adj[i].len() >= 3;
        }
        self.edge_veh.resize(self.edges.len(), vec![]);
        self.cross_cnt.resize(self.edges.len(), 0);
    }

    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for n in self.nodes.iter().filter(|n| n.active || n.island) {
            b.0 = b.0.min(n.x);
            b.1 = b.1.min(n.y);
            b.2 = b.2.max(n.x);
            b.3 = b.3.max(n.y);
        }
        if b.0 > b.2 {
            return (0.0, 0.0, 100.0, 100.0);
        }
        b
    }

    pub fn vehicle_pose(&self, v: &Vehicle) -> (f64, f64, f64) {
        let e = &self.edges[v.edge];
        e.point(
            &self.nodes,
            (v.pos / e.length).clamp(0.0, 1.0),
            e.base_offset(v.lane),
        )
    }

    pub fn rear_in_lane(&self, e: usize, lane: u8) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for &j in &self.edge_veh[e] {
            let o = &self.vehicles[j as usize];
            if o.alive && o.lane == lane && best.is_none_or(|b| o.pos < b.0) {
                best = Some((o.pos, o.v));
            }
        }
        best
    }

    pub fn spawn_traffic(&mut self) {
        if self.buildings.len() < 2 {
            return;
        }

        let b_orig = self.rng.range(self.buildings.len());
        let b_dest = self.rng.range(self.buildings.len());
        if b_orig == b_dest {
            return;
        }

        let start_n = self.buildings[b_orig].node;
        let dest_n = self.buildings[b_dest].node;
        if start_n == dest_n {
            return;
        }

        let Some(path) = self.astar(start_n, dest_n) else {
            return;
        };

        let first = path[0];
        let nl = self.edges[first].lanes;
        let start_lane = self.rng.range(nl as usize) as u8;

        let mut lane = None;
        for t in 0..nl {
            let ln = (start_lane + t) % nl;
            let rp = self.rear_in_lane(first, ln).map_or(f64::INFINITY, |r| r.0);
            if rp >= CAR_LEN + MIN_GAP + 1.0 {
                lane = Some(ln);
                break;
            }
        }

        let Some(lane) = lane else {
            self.blocked += 1;
            return;
        };

        if self.buildings[b_orig].cars_parked > 0 {
            self.buildings[b_orig].cars_parked -= 1;
        }

        let id = self.next_id;
        self.next_id += 1;
        let factor = self.rng.between(0.85, 1.15);

        let citizen_id = self
            .citizens
            .iter()
            .position(|c| c.current_b == b_orig && c.state == CitizenState::Dwelling)
            .unwrap_or(0) as u32;

        self.vehicles.push(Vehicle {
            id,
            edge: first,
            lane,
            pos: 0.0,
            v: self.edges[first].vmax * 0.5,
            a: 0.0,
            path,
            pi: 0,
            dest: dest_n,
            wait: 0.0,
            v0_factor: factor,
            alive: true,
            citizen: citizen_id,
            dest_b: b_dest as u32,
        });

        if (citizen_id as usize) < self.citizens.len() {
            self.citizens[citizen_id as usize].state = CitizenState::TravelingCar;
        }

        self.edge_veh[first].push((self.vehicles.len() - 1) as u32);
        self.trips_car += 1;
        self.spawned += 1;
    }

    pub fn start_car_trip(&mut self, citizen_idx: usize, dest_b: usize) -> bool {
        let cur_b = self.citizens[citizen_idx].current_b;
        let start_n = self.buildings[cur_b].node;
        let dest_n = self.buildings[dest_b].node;

        if start_n == dest_n {
            return false;
        }

        let Some(path) = self.astar(start_n, dest_n) else {
            return false;
        };

        let first = path[0];
        let nl = self.edges[first].lanes;
        let start_lane = self.rng.range(nl as usize) as u8;

        let mut lane = None;
        for t in 0..nl {
            let ln = (start_lane + t) % nl;
            let rp = self.rear_in_lane(first, ln).map_or(f64::INFINITY, |r| r.0);
            if rp >= CAR_LEN + MIN_GAP + 1.0 {
                lane = Some(ln);
                break;
            }
        }

        let Some(lane) = lane else {
            self.blocked += 1;
            return false;
        };

        if self.buildings[cur_b].cars_parked > 0 {
            self.buildings[cur_b].cars_parked -= 1;
        }

        let id = self.next_id;
        self.next_id += 1;
        let factor = self.rng.between(0.85, 1.15);

        self.vehicles.push(Vehicle {
            id,
            edge: first,
            lane,
            pos: 0.0,
            v: self.edges[first].vmax * 0.5,
            a: 0.0,
            path,
            pi: 0,
            dest: dest_n,
            wait: 0.0,
            v0_factor: factor,
            alive: true,
            citizen: citizen_idx as u32,
            dest_b: dest_b as u32,
        });

        self.edge_veh[first].push((self.vehicles.len() - 1) as u32);
        self.citizens[citizen_idx].state = CitizenState::TravelingCar;
        self.trips_car += 1;

        true
    }

    pub fn arrive_car(&mut self, citizen_idx: usize, dest_b: usize) {
        if citizen_idx < self.citizens.len() {
            self.citizens[citizen_idx].state = CitizenState::Dwelling;
            self.citizens[citizen_idx].current_b = dest_b;
            let dwell_base = self.rng.between(8.0, 22.0) / (self.spawn_rate * 0.25).max(1.0);
            self.citizens[citizen_idx].dwell_left = dwell_base * self.dwell_scale;
        }

        if dest_b < self.buildings.len() {
            let cap = self.buildings[dest_b].parking_cap + 20;
            self.buildings[dest_b].cars_parked = (self.buildings[dest_b].cars_parked + 1).min(cap);
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.time += dt;
        self.step_count += 1;
        let ne = self.edges.len();
        if self.edge_veh.len() != ne {
            self.edge_veh.resize(ne, vec![]);
        }
        if self.cross_cnt.len() != ne {
            self.cross_cnt.resize(ne, 0);
        }
        for l in self.edge_veh.iter_mut() {
            l.clear();
        }
        for (i, v) in self.vehicles.iter().enumerate() {
            self.edge_veh[v.edge].push(i as u32);
        }
        {
            let veh = &self.vehicles;
            for l in self.edge_veh.iter_mut() {
                l.sort_unstable_by(|a, b| {
                    veh[*b as usize]
                        .pos
                        .partial_cmp(&veh[*a as usize].pos)
                        .unwrap()
                });
            }
        }
        for c in self.cross_cnt.iter_mut() {
            *c = 0;
        }
        for p in &self.peds {
            if p.phase == PPhase::Cross {
                self.cross_cnt[p.seg_e] += 1;
                if let Some(r) = self.edges[p.seg_e].rev {
                    self.cross_cnt[r] += 1;
                }
            }
        }

        let nv = self.vehicles.len();
        let mut acc = vec![0.0f64; nv];
        let mut lead = vec![-1i32; nv];
        for e in 0..ne {
            let edge = &self.edges[e];
            let list = &self.edge_veh[e];
            for k in 0..list.len() {
                let i = list[k] as usize;
                let v = &self.vehicles[i];
                let mut gap = f64::INFINITY;
                let mut lv = 0.0;
                for kk in (0..k).rev() {
                    let j = list[kk] as usize;
                    let o = &self.vehicles[j];
                    if o.lane == v.lane {
                        gap = o.pos - v.pos - CAR_LEN;
                        lv = o.v;
                        lead[i] = j as i32;
                        break;
                    }
                }
                if lead[i] < 0 {
                    let dist = edge.length - v.pos;
                    if dist < 60.0 {
                        let node = &self.nodes[edge.to];
                        let stop =
                            (node.signal && is_red(self.time, node, edge)) || self.cross_cnt[e] > 0;
                        if stop && v.v * v.v / (2.0 * B_STOP) <= dist - 0.5 {
                            gap = gap.min(dist - 1.0);
                            lv = 0.0;
                        }
                        if v.pi + 1 < v.path.len() {
                            let nxt = v.path[v.pi + 1];
                            let nl = v.lane.min(self.edges[nxt].lanes - 1);
                            if let Some((rp, rv)) = self.rear_in_lane(nxt, nl) {
                                let g2 = dist + rp - CAR_LEN;
                                if g2 < gap {
                                    gap = g2;
                                    lv = rv;
                                }
                            }
                        }
                    }
                }
                let v0 = edge.vmax * v.v0_factor;
                let free = 1.0 - (v.v / v0).powi(4);
                let a = if gap.is_finite() {
                    let s = gap.max(0.1);
                    let sstar = S0
                        + (v.v * T_HEAD + v.v * (v.v - lv) / (2.0 * (A_MAX * B_COMF).sqrt()))
                            .max(0.0);
                    A_MAX * (free - (sstar / s).powi(2))
                } else {
                    A_MAX * free
                };
                acc[i] = a.clamp(-9.0, A_MAX);
            }
        }

        let mut arrivals: Vec<(u32, u32)> = vec![];
        for e in 0..ne {
            let len = self.edges[e].length;
            let mut k = 0;
            while k < self.edge_veh[e].len() {
                let i = self.edge_veh[e][k] as usize;
                k += 1;
                if !self.vehicles[i].alive {
                    continue;
                }
                let mut np;
                {
                    let lp = if lead[i] >= 0 {
                        Some(self.vehicles[lead[i] as usize].pos)
                    } else {
                        None
                    };
                    let v = &mut self.vehicles[i];
                    v.a = acc[i];
                    v.v = (v.v + acc[i] * dt).max(0.0);
                    np = v.pos + v.v * dt;
                    if let Some(lp) = lp {
                        np = np.min(lp - CAR_LEN - 0.2).max(v.pos);
                        v.v = ((np - v.pos) / dt).min(v.v);
                    }
                }
                if np >= len && lead[i] < 0 {
                    let (last, nxt, lane) = {
                        let v = &self.vehicles[i];
                        let last = v.pi + 1 >= v.path.len();
                        (last, if last { 0 } else { v.path[v.pi + 1] }, v.lane)
                    };
                    if last {
                        if self.cross_cnt[e] > 0 {
                            let v = &mut self.vehicles[i];
                            v.pos = len;
                            v.v = 0.0;
                        } else {
                            let v = &mut self.vehicles[i];
                            v.alive = false;
                            arrivals.push((v.citizen, v.dest_b));
                            self.arrived += 1;
                            self.arr_window += 1;
                        }
                        continue;
                    }
                    let over = np - len;
                    let nl = self.edges[nxt].lanes;
                    let pref = lane.min(nl - 1);
                    let need = CAR_LEN
                        + MIN_GAP
                        + if self.edges[nxt].ring && !self.edges[e].ring {
                            8.0
                        } else {
                            0.0
                        };
                    let mut chosen = None;
                    if self.cross_cnt[nxt] == 0 {
                        for t in 0..nl {
                            let ln = (pref + t) % nl;
                            let rp = self.rear_in_lane(nxt, ln).map_or(f64::INFINITY, |r| r.0);
                            if rp - over >= need {
                                chosen = Some(ln);
                                break;
                            }
                        }
                    }
                    let v = &mut self.vehicles[i];
                    if let Some(ln) = chosen {
                        v.edge = nxt;
                        v.pi += 1;
                        v.pos = over;
                        v.lane = ln;
                        self.edge_veh[nxt].push(i as u32);
                    } else {
                        v.pos = len;
                        v.v = 0.0;
                    }
                } else {
                    self.vehicles[i].pos = np.min(len);
                }
            }
        }

        let alpha = 1.0 - (-dt / 8.0).exp();
        for e in 0..ne {
            let n = self.edge_veh[e].len();
            let mut sum = 0.0;
            for &j in &self.edge_veh[e] {
                sum += self.vehicles[j as usize].v;
            }
            let edge = &mut self.edges[e];
            edge.count = n;
            edge.avg_speed = if n > 0 { sum / n as f64 } else { edge.vmax };
            let dens = n as f64 * (CAR_LEN + MIN_GAP) / (edge.length * edge.lanes as f64);
            let raw = (1.0 - edge.avg_speed / edge.vmax).clamp(0.0, 1.0) * (dens * 2.0).min(1.0);
            edge.jam += alpha * (raw - edge.jam);
            edge.tt_speed += alpha * (edge.avg_speed - edge.tt_speed);
        }
        let mut sv = 0.0;
        let mut sv0 = 0.0;
        for v in self.vehicles.iter_mut() {
            let v0 = self.edges[v.edge].vmax * v.v0_factor;
            sv += v.v;
            sv0 += v0;
            if v.v < 0.5 {
                v.wait += dt;
            } else if v.v > 2.0 {
                v.wait = 0.0;
            }
            if v.wait > 150.0 && v.alive {
                v.alive = false;
                arrivals.push((v.citizen, v.dest_b));
                self.teleported += 1;
                self.removed += 1;
            }
        }
        self.avg_speed = if self.vehicles.is_empty() {
            0.0
        } else {
            sv / self.vehicles.len() as f64
        };
        self.avg_speed_ratio = if sv0 > 0.0 { sv / sv0 } else { 1.0 };
        for (c, b) in arrivals {
            self.arrive_car(c as usize, b as usize);
        }
        self.pending_arrivals.clear();

        let mut jammed = 0;
        for e in &self.edges {
            if e.jam > JAM_EDGE {
                jammed += 1;
            }
        }
        self.jammed_edges = jammed;
        let mut mx: f64 = 0.0;
        for i in 0..self.nodes.len() {
            let ins = &self.in_adj[i];
            let j = if ins.is_empty() || !self.nodes[i].active {
                0.0
            } else {
                let m = ins.iter().map(|&e| self.edges[e].jam).fold(0.0, f64::max);
                let a = ins.iter().map(|&e| self.edges[e].jam).sum::<f64>() / ins.len() as f64;
                0.6 * m + 0.4 * a
            };
            self.nodes[i].jam = j;
            mx = mx.max(j);
        }
        self.max_node_jam = mx;

        self.spawn_acc += self.spawn_rate * self.spawn_scale * dt;
        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;
            self.spawn_traffic();
        }

        self.update_citizens(dt);
        self.update_peds(dt);

        let nv = self.vehicles.len();
        let mut i = (self.step_count % 100) as usize;
        while i < nv {
            self.reroute_vehicle(i);
            i += 100;
        }
        self.vehicles.retain(|v| v.alive);

        self.window_t += dt;
        if self.window_t >= 5.0 {
            let r = self.arr_window as f64 / self.window_t * 60.0;
            self.flow_per_min = if self.flow_per_min == 0.0 {
                r
            } else {
                0.5 * self.flow_per_min + 0.5 * r
            };
            self.arr_window = 0;
            self.window_t = 0.0;
        }
        self.hist_t += dt;
        if self.hist_t >= 2.0 {
            self.hist_t = 0.0;
            self.hist.push([
                self.citizens.len() as f32,
                self.vehicles.len() as f32,
                (self.avg_speed * 3.6) as f32,
            ]);
            if self.hist.len() > 150 {
                self.hist.remove(0);
            }
        }
    }
}
