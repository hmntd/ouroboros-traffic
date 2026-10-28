use crate::citizen::CitizenState;
use crate::graph::LANE_W;
use crate::world::World;

pub const CROSSING_DIST: f64 = 9.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PPhase {
    Walk,
    WaitCross,
    Cross,
}

#[derive(Clone, Debug)]
pub struct Ped {
    pub id: u32,
    pub citizen: u32,
    pub dest_b: u32,
    pub phase: PPhase,
    pub path: Vec<usize>,
    pub pi: usize,
    pub seg_e: usize,
    pub t: f64,
    pub speed: f64,
    pub x: f64,
    pub y: f64,
    pub wait: f64,
    pub alive: bool,
}

impl World {
    pub fn start_walk_trip(&mut self, citizen_idx: usize, dest_b: usize) -> bool {
        let cur_b = self.citizens[citizen_idx].current_b;
        let start_n = self.buildings[cur_b].node;
        let dest_n = self.buildings[dest_b].node;

        let (path, seg_e) = if start_n == dest_n {
            (vec![start_n], 0)
        } else {
            let Some(p) = self.ped_path(start_n, dest_n) else {
                return false;
            };
            let seg_e = self.out_adj[p[0]]
                .iter()
                .copied()
                .find(|&e| self.edges[e].to == p[1])
                .unwrap_or_else(|| self.out_adj[p[0]][0]);
            (p, seg_e)
        };

        let (bx, by) = (self.buildings[cur_b].x, self.buildings[cur_b].y);
        let id = self.next_id;
        self.next_id += 1;

        self.peds.push(Ped {
            id,
            citizen: citizen_idx as u32,
            dest_b: dest_b as u32,
            phase: PPhase::Walk,
            path,
            pi: 0,
            seg_e,
            t: 0.0,
            speed: self.rng.between(1.2, 1.5),
            x: bx,
            y: by,
            wait: 0.0,
            alive: true,
        });

        self.citizens[citizen_idx].state = CitizenState::TravelingWalk;
        self.trips_walk += 1;
        true
    }

    pub fn update_peds(&mut self, dt: f64) {
        let mut completed: Vec<(u32, u32)> = vec![];

        for p in self.peds.iter_mut() {
            if !p.alive {
                continue;
            }

            match p.phase {
                PPhase::Walk => {
                    if p.path.len() <= 1 || p.pi + 1 >= p.path.len() {
                        let dest_b = &self.buildings[p.dest_b as usize];
                        let dx = dest_b.x - p.x;
                        let dy = dest_b.y - p.y;
                        let d = dx.hypot(dy);
                        if d < 3.0 {
                            p.alive = false;
                            completed.push((p.citizen, p.dest_b));
                        } else {
                            p.x += (dx / d) * p.speed * dt;
                            p.y += (dy / d) * p.speed * dt;
                        }
                        continue;
                    }

                    let edge = &self.edges[p.seg_e];
                    let seg_len = edge.length.max(1.0);
                    p.t += (p.speed * dt) / seg_len;

                    let sidewalk_off = edge.center_offset() + 5.5;
                    let (px, py, _) = edge.point(&self.nodes, p.t.clamp(0.0, 1.0), sidewalk_off);
                    p.x = px;
                    p.y = py;

                    if p.t >= 0.98 {
                        p.pi += 1;
                        if p.pi + 1 < p.path.len() {
                            let next_u = p.path[p.pi];
                            let next_v = p.path[p.pi + 1];
                            let next_e = self.out_adj[next_u]
                                .iter()
                                .copied()
                                .find(|&e| self.edges[e].to == next_v)
                                .unwrap_or_else(|| self.out_adj[next_u][0]);
                            p.seg_e = next_e;
                            p.phase = PPhase::WaitCross;
                            p.wait = self.rng.between(0.5, 1.8);
                        } else {
                            p.t = 0.0;
                        }
                    }
                }
                PPhase::WaitCross => {
                    p.wait -= dt;
                    if p.wait <= 0.0 {
                        p.phase = PPhase::Cross;
                        p.t = 0.0;
                    }
                }
                PPhase::Cross => {
                    let edge = &self.edges[p.seg_e];
                    let width = (edge.lanes as f64 * LANE_W).max(LANE_W);
                    let cross_time = (width / p.speed).max(2.2);

                    p.t += dt / cross_time;

                    let cross_pos = 1.0 - (CROSSING_DIST / edge.length).clamp(0.05, 0.4);
                    let side_a = edge.center_offset() + 5.5;
                    let side_b = -(edge.center_offset() + 5.5);
                    let cur_off = side_a + (side_b - side_a) * p.t.clamp(0.0, 1.0);

                    let (px, py, _) = edge.point(&self.nodes, cross_pos, cur_off);
                    p.x = px;
                    p.y = py;

                    if p.t >= 1.0 {
                        p.phase = PPhase::Walk;
                        p.t = 0.0;
                    }
                }
            }
        }

        for (c, b) in completed {
            let c_idx = c as usize;
            let b_idx = b as usize;
            if c_idx < self.citizens.len() && b_idx < self.buildings.len() {
                self.citizens[c_idx].state = CitizenState::Dwelling;
                self.citizens[c_idx].current_b = b_idx;
                let dwell_base = self.rng.between(8.0, 22.0) / (self.spawn_rate * 0.25).max(1.0);
                self.citizens[c_idx].dwell_left = dwell_base * self.dwell_scale;
            }
        }

        self.peds.retain(|p| p.alive);
    }

    pub fn reroute_peds_through(&mut self, node: usize) {
        let n_peds = self.peds.len();
        for i in 0..n_peds {
            if self.peds[i].path.contains(&node) {
                let dest_n = self.buildings[self.peds[i].dest_b as usize].node;
                let cur_n = self.peds[i].path[self.peds[i].pi.min(self.peds[i].path.len() - 1)];
                if let Some(new_p) = self.ped_path(cur_n, dest_n) {
                    self.peds[i].path = new_p;
                    self.peds[i].pi = 0;
                    self.peds[i].phase = PPhase::Walk;
                    self.peds[i].t = 0.0;
                }
            }
        }
    }
}
