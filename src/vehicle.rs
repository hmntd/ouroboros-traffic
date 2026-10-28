pub const CAR_LEN: f64 = 4.5;
pub const MIN_GAP: f64 = 2.0;
pub const A_MAX: f64 = 1.6;
pub const B_COMF: f64 = 2.2;
pub const T_HEAD: f64 = 1.3;
pub const S0: f64 = 2.0;
pub const B_STOP: f64 = 5.0;
pub const VMAX_GLOBAL: f64 = 22.0;

#[derive(Clone)]
pub struct Vehicle {
    pub id: u32,
    pub edge: usize,
    pub lane: u8,
    pub pos: f64,
    pub v: f64,
    pub a: f64,
    pub path: Vec<usize>,
    pub pi: usize,
    pub dest: usize,
    pub wait: f64,
    pub v0_factor: f64,
    pub alive: bool,
    pub citizen: u32,
    pub dest_b: u32,
}
