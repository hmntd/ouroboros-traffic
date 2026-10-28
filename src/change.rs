use crate::world::World;

#[derive(Clone, Debug)]
pub struct Change {
    pub id: u32,
    pub time: f64,
    pub kind: u8, // 0: Expand, 1: Widen, 2: Roundabout, 3: Bypass
    pub title: String,
    pub desc: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub edges: Vec<usize>,
    pub nodes: Vec<usize>,
}

impl World {
    pub fn record_change(
        &mut self,
        kind: u8,
        title: String,
        desc: String,
        x: f64,
        y: f64,
        radius: f64,
        edges: Vec<usize>,
        nodes: Vec<usize>,
    ) {
        let id = self.changes.len() as u32 + 1;
        self.changes.push(Change {
            id,
            time: self.time,
            kind,
            title,
            desc,
            x,
            y,
            radius,
            edges,
            nodes,
        });
    }
}
