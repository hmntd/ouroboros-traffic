use ouroboros_traffic::planner::Planner;
use ouroboros_traffic::sim::World;

#[test]
fn headless_ouroboros_run() {
    let mut w = World::generate(7, 20, 14);
    let mut p = Planner::new();
    let dt = 0.1;
    let mut last_log = 0;
    let secs: f64 = std::env::var("SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(240.0);
    let mut t = 0.0;
    while t < secs {
        w.step(dt);
        p.update(&mut w, dt, 3);
        t += dt;
        if (t * 10.0).round() as u64 % 600 == 0 {
            println!(
                "t={:4.0} veh={:5} rate={:.2} spd={:.1}km/h jam_edges={:3} maxjam={:.2} blocked={} removed={} phase={:?} mods={:?}",
                t, w.vehicles.len(), w.spawn_rate, w.avg_speed * 3.6, w.jammed_edges, w.max_node_jam,
                w.blocked, w.removed, p.phase, w.mods
            );
        }
        while last_log < p.log.len() {
            println!("   LOG {}", p.log[last_log]);
            last_log += 1;
        }
    }
    assert!(w.vehicles.iter().all(|v| v.pos.is_finite() && v.pos >= 0.0));
}
