pub mod building;
pub mod change;
pub mod citizen;
pub mod city;
pub mod expansion;
pub mod graph;
pub mod modifier;
pub mod pedestrian;
pub mod planner;
pub mod render;
pub mod rng;
pub mod routing;
pub mod sim;
pub mod vehicle;
pub mod world;

use planner::Planner;
use render::{Camera, ViewOpts};
use sim::World;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, window};

#[wasm_bindgen]
pub struct Game {
    world: World,
    planner: Planner,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    cam: Camera,
    opts: ViewOpts,
    speed: f64,
    paused: bool,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str, seed: u32) -> Result<Game, JsValue> {
        let document = window()
            .ok_or("no window")?
            .document()
            .ok_or("no document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("canvas not found")?
            .dyn_into::<HtmlCanvasElement>()?;
        let ctx = canvas
            .get_context("2d")?
            .ok_or("no 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let world = World::generate(seed as u64, 4, 3);
        let cam = Camera::new(&world);
        let mut planner = Planner::new();
        planner.push_log(
            0.0,
            "Settlement founded. Citizens commuting between homes and workplaces.".into(),
        );

        Ok(Game {
            world,
            planner,
            canvas,
            ctx,
            cam,
            opts: ViewOpts {
                show_nodes: false,
                show_paths: false,
                show_heat: true,
            },
            speed: 1.0,
            paused: false,
        })
    }

    pub fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }
        let total = dt.clamp(0.0, 0.1) * self.speed;
        let n = ((total / 0.05).ceil() as usize).clamp(1, 60);
        let sub = total / n as f64;
        for _ in 0..n {
            self.world.step(sub);
            self.planner.update(&mut self.world, sub, 3);
        }
    }

    pub fn draw(&self) {
        let (w, h) = (self.canvas.width() as f64, self.canvas.height() as f64);
        render::draw(&self.ctx, w, h, &self.world, &self.cam, self.opts);
    }

    pub fn set_speed(&mut self, s: f64) {
        self.speed = s.clamp(0.0, 20.0);
    }
    pub fn set_paused(&mut self, p: bool) {
        self.paused = p;
    }
    pub fn set_show_nodes(&mut self, v: bool) {
        self.opts.show_nodes = v;
    }
    pub fn set_show_paths(&mut self, v: bool) {
        self.opts.show_paths = v;
    }
    pub fn set_show_heat(&mut self, v: bool) {
        self.opts.show_heat = v;
    }
    pub fn set_solver_enabled(&mut self, v: bool) {
        self.planner.enabled = v;
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.cam.px += dx;
        self.cam.py += dy;
    }

    pub fn zoom_at(&mut self, factor: f64, mx: f64, my: f64) {
        let (w, h) = (self.canvas.width() as f64, self.canvas.height() as f64);
        let s0 = self.cam.scale(w, h);
        let z = (self.cam.zoom * factor).clamp(0.4, 12.0);
        self.cam.zoom = z;
        let s1 = self.cam.scale(w, h);
        let wx = (mx - w / 2.0 - self.cam.px) / s0 + self.cam.cx;
        let wy = (my - h / 2.0 - self.cam.py) / s0 + self.cam.cy;
        self.cam.px = mx - w / 2.0 - (wx - self.cam.cx) * s1;
        self.cam.py = my - h / 2.0 - (wy - self.cam.cy) * s1;
    }

    pub fn reset_view(&mut self) {
        self.cam.zoom = 1.0;
        self.cam.px = 0.0;
        self.cam.py = 0.0;
        let b = self.world.bounds();
        self.cam.cx = (b.0 + b.2) / 2.0;
        self.cam.cy = (b.1 + b.3) / 2.0;
    }

    pub fn reset(&mut self, seed: u32) {
        self.world = World::generate(seed as u64, 4, 3);
        self.cam = Camera::new(&self.world);
        self.planner = Planner::new();
        self.planner.push_log(
            0.0,
            "New settlement founded. City will grow over time.".into(),
        );
    }

    pub fn focus_change(&mut self, id: u32) {
        if let Some(c) = self.world.changes.iter().find(|c| c.id == id).cloned() {
            let (cx, cy, r) = (c.x, c.y, c.radius);
            self.world.focus = Some((cx, cy, r, self.world.time));
            let (w, h) = (self.canvas.width() as f64, self.canvas.height() as f64);
            let base_scale = (w / self.cam.bw).min(h / self.cam.bh);
            let target_zoom = (w.min(h) / (r * 2.4 * base_scale)).clamp(0.7, 3.5);
            self.cam.center_on(cx, cy, target_zoom);
        }
    }

    pub fn changes_json(&self) -> String {
        let items: Vec<String> = self
            .world
            .changes
            .iter()
            .rev()
            .map(|c| {
                format!(
                    "{{\"id\":{},\"time\":{:.1},\"kind\":{},\"title\":\"{}\",\"desc\":\"{}\",\"x\":{:.1},\"y\":{:.1},\"radius\":{:.1}}}",
                    c.id,
                    c.time,
                    c.kind,
                    c.title.replace('"', "\\\""),
                    c.desc.replace('"', "\\\""),
                    c.x,
                    c.y,
                    c.radius
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    pub fn stats_json(&self) -> String {
        let w = &self.world;
        let p = &self.planner;
        let hist = |k: usize| {
            w.hist
                .iter()
                .map(|h| format!("{:.1}", h[k]))
                .collect::<Vec<_>>()
                .join(",")
        };
        let log = p
            .log
            .iter()
            .rev()
            .take(12)
            .map(|s| format!("\"{}\"", s.replace('"', "'")))
            .collect::<Vec<_>>()
            .join(",");
        let active_nodes = w.nodes.iter().filter(|n| n.active).count();
        let queue: usize = w
            .edges
            .iter()
            .filter(|e| e.jam > sim::JAM_EDGE)
            .map(|e| e.count)
            .sum();
        let total_parked: usize = w.buildings.iter().map(|b| b.cars_parked).sum();

        format!(
            "{{\"time\":{:.1},\"vehicles\":{},\"citizens\":{},\"peds\":{},\"buildings\":{},\"parked\":{},\"tripsCar\":{},\"tripsWalk\":{},\"spawnRate\":{:.2},\"avgSpeed\":{:.1},\"speedRatio\":{:.3},\"flow\":{:.0},\"jammedEdges\":{},\"maxJam\":{:.3},\"queued\":{},\"blocked\":{},\"removed\":{},\"arrived\":{},\"phase\":\"{}\",\"progress\":{:.3},\"candidates\":{},\"generation\":{},\"mods\":[{},{},{}],\"expansions\":{},\"changesCount\":{},\"isMaxCity\":{},\"maxNodes\":{},\"nodes\":{},\"edges\":{},\"hVeh\":[{}],\"hSpeed\":[{}],\"hFlow\":[{}],\"log\":[{}]}}",
            w.time,
            w.vehicles.len(),
            w.citizens.len(),
            w.peds.len(),
            w.buildings.len(),
            total_parked,
            w.trips_car,
            w.trips_walk,
            w.spawn_rate,
            w.avg_speed * 3.6,
            w.avg_speed_ratio,
            w.flow_per_min,
            w.jammed_edges,
            w.max_node_jam,
            queue,
            w.blocked,
            w.removed,
            w.arrived,
            p.phase.name(),
            p.progress(),
            p.candidates_count(),
            p.generation,
            w.mods[0],
            w.mods[1],
            w.mods[2],
            w.expansions,
            w.changes.len(),
            w.is_max_city(),
            crate::expansion::MAX_CITY_NODES,
            active_nodes,
            w.edges.len(),
            hist(0),
            hist(1),
            hist(2),
            log
        )
    }
}
