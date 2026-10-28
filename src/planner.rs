use crate::sim::{Mod, World};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Growing,
    Analyzing,
    Recovering,
}

impl Phase {
    pub fn name(&self) -> &'static str {
        match self {
            Phase::Growing => "Growing demand",
            Phase::Analyzing => "Shadow simulation",
            Phase::Recovering => "Recovering",
        }
    }
}

struct Shadow {
    world: World,
    modif: Option<Mod>,
    label: String,
    steps_left: u32,
    arr0: u64,
    blk0: u64,
    rem0: u64,
    ratio_sum: f64,
}

impl Shadow {
    fn score(&self) -> f64 {
        let w = &self.world;
        let mean_ratio = self.ratio_sum / (SHADOW_STEPS as f64);
        (w.arrived - self.arr0) as f64 + 150.0 * mean_ratio
            - 0.3 * w.jammed_edges as f64
            - 0.6 * (w.blocked - self.blk0) as f64
            - 2.0 * (w.removed - self.rem0) as f64
            - 0.4 * w.vehicles.len() as f64
    }
}

const SHADOW_DT: f64 = 0.1;
const SHADOW_STEPS: u32 = 400;
const JAM_EDGE_COUNT: usize = 8;
const JAM_HOLD: f64 = 8.0;

pub struct Planner {
    pub phase: Phase,
    pub generation: u32,
    pub growth: f64,
    pub enabled: bool,
    jam_timer: f64,
    timer: f64,
    calm_timer: f64,
    expand_timer: f64,
    shadows: Vec<Shadow>,
    pub log: Vec<String>,
    pub last_event_time: f64,
}

impl Planner {
    pub fn new() -> Self {
        Planner {
            phase: Phase::Growing,
            generation: 1,
            growth: 0.03,
            enabled: true,
            jam_timer: 0.0,
            timer: 0.0,
            calm_timer: 0.0,
            expand_timer: 0.0,
            shadows: vec![],
            log: vec![],
            last_event_time: -100.0,
        }
    }

    pub fn push_log(&mut self, t: f64, msg: String) {
        let s = format!("[{:02}:{:02}] {}", (t / 60.0) as u32, (t as u32) % 60, msg);
        self.log.push(s);
        if self.log.len() > 40 {
            self.log.remove(0);
        }
        self.last_event_time = t;
    }

    pub fn progress(&self) -> f64 {
        if self.shadows.is_empty() {
            return 0.0;
        }
        let done: u32 = self
            .shadows
            .iter()
            .map(|s| SHADOW_STEPS - s.steps_left)
            .sum();
        done as f64 / (SHADOW_STEPS as usize * self.shadows.len()) as f64
    }

    pub fn candidates_count(&self) -> usize {
        self.shadows.len()
    }

    fn jammed(w: &World) -> bool {
        let threshold = (JAM_EDGE_COUNT).min((w.edges.len() / 6).max(3));
        (w.jammed_edges >= threshold && w.max_node_jam > 0.28) || w.max_node_jam > 0.48
    }

    pub fn update(&mut self, w: &mut World, dt: f64, budget: u32) {
        if !self.enabled {
            return;
        }
        match self.phase {
            Phase::Growing => {
                let growth_factor = 1.0 + (self.generation as f64 - 1.0) * 0.15;
                w.spawn_rate += self.growth * growth_factor * dt;
                self.expand_timer += dt;
                if self.expand_timer >= 75.0 {
                    self.expand_timer = 0.0;
                    if w.expand_city() {
                        let t = w.time;
                        self.push_log(
                            t,
                            format!(
                                "Natural urban expansion: now {} nodes, {} buildings, {} citizens.",
                                w.nodes.len(),
                                w.buildings.len(),
                                w.citizens.len()
                            ),
                        );
                    }
                }
                if Self::jammed(w) {
                    self.jam_timer += dt;
                } else {
                    self.jam_timer = (self.jam_timer - 0.5 * dt).max(0.0);
                }
                if self.jam_timer >= JAM_HOLD {
                    self.begin_analysis(w);
                }
            }
            Phase::Analyzing => {
                for s in self.shadows.iter_mut() {
                    for _ in 0..budget.min(s.steps_left) {
                        s.world.step(SHADOW_DT);
                        s.ratio_sum += s.world.avg_speed_ratio;
                        s.steps_left -= 1;
                    }
                }
                if self.shadows.iter().all(|s| s.steps_left == 0) {
                    self.finish_analysis(w);
                }
            }
            Phase::Recovering => {
                self.timer += dt;
                if Self::jammed(w) {
                    self.calm_timer = 0.0;
                } else {
                    self.calm_timer += dt;
                }
                if (self.timer > 15.0 && self.calm_timer > 10.0) || self.timer > 75.0 {
                    if self.calm_timer > 10.0 {
                        self.generation += 1;
                        let t = w.time;
                        let msg = format!(
                            "Flow restored. Generation {} - demand {:.1} veh/s",
                            self.generation, w.spawn_rate
                        );
                        self.push_log(t, msg);

                        if w.expand_city() {
                            self.expand_timer = 0.0;
                            let exp_msg = format!(
                                "City expanded! Now {} nodes, {} buildings, {} citizens.",
                                w.nodes.len(),
                                w.buildings.len(),
                                w.citizens.len()
                            );
                            self.push_log(t, exp_msg);
                        }
                    }
                    self.phase = Phase::Growing;
                    w.spawn_scale = 1.0;
                    w.trip_scale = 1.0;
                    self.jam_timer = 0.0;
                }
            }
        }
    }

    fn begin_analysis(&mut self, w: &mut World) {
        let worst = w.worst_nodes(3);
        let mut cands: Vec<Mod> = vec![];
        for &n in &worst {
            if w.can_widen(n) {
                cands.push(Mod::Widen(n));
            }
            if w.can_roundabout(n) {
                cands.push(Mod::Roundabout(n));
            }
            if let Some(b) = w.bypass_candidate(n) {
                cands.push(b);
            }
        }
        let t = w.time;
        let msg = format!(
            "JAM detected near node {} ({} congested roads). Testing {} options...",
            worst.first().copied().unwrap_or(0),
            w.jammed_edges,
            cands.len()
        );
        self.push_log(t, msg);
        self.shadows.clear();
        let mk = |m: Option<Mod>, label: String| {
            let mut sw = w.clone();
            if let Some(m) = &m {
                sw.apply_mod(m);
            }
            Shadow {
                arr0: sw.arrived,
                blk0: sw.blocked,
                rem0: sw.removed,
                ratio_sum: 0.0,
                world: sw,
                modif: m,
                label,
                steps_left: SHADOW_STEPS,
            }
        };
        self.shadows.push(mk(None, "do nothing".into()));
        for c in cands {
            let label = w.describe(&c);
            self.shadows.push(mk(Some(c), label));
        }
        w.spawn_scale = 0.35;
        w.trip_scale = 0.35;
        self.phase = Phase::Analyzing;
    }

    fn finish_analysis(&mut self, w: &mut World) {
        let base = self.shadows[0].score();
        let mut best: Option<(f64, usize)> = None;
        let mut report = vec![];
        for (i, s) in self.shadows.iter().enumerate() {
            let sc = s.score();
            report.push(format!("{}: {:.0}", s.label, sc));
            if i > 0 && best.is_none_or(|b| sc > b.0) {
                best = Some((sc, i));
            }
        }
        let t = w.time;
        self.push_log(t, format!("Scores - {}", report.join(" | ")));
        let margin = (base.abs() * 0.01).max(1.5);
        match best {
            Some((sc, i)) if sc > base + margin => {
                let m = self.shadows[i].modif.clone().unwrap();
                let label = self.shadows[i].label.clone();
                w.apply_mod(&m);
                self.push_log(
                    t,
                    format!("BUILD: {} (score {:.0} vs {:.0})", label, sc, base),
                );
            }
            _ => {
                w.spawn_rate *= 0.85;
                self.push_log(
                    t,
                    format!(
                        "No modification beat baseline. Throttling demand to {:.1} veh/s",
                        w.spawn_rate
                    ),
                );
            }
        }
        self.shadows.clear();
        self.phase = Phase::Recovering;
        self.timer = 0.0;
        self.calm_timer = 0.0;
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}
