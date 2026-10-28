use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CitizenState {
    Dwelling,
    TravelingCar,
    TravelingWalk,
}

#[derive(Clone, Debug)]
pub struct Citizen {
    pub id: u32,
    pub home_b: usize,
    pub work_b: usize,
    pub current_b: usize,
    pub state: CitizenState,
    pub dwell_left: f64,
    pub has_car: bool,
}

impl World {
    pub fn populate_citizens(&mut self, count: usize) {
        let res_ids: Vec<usize> = self
            .buildings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == 0)
            .map(|(i, _)| i)
            .collect();
        let work_ids: Vec<usize> = self
            .buildings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind != 0)
            .map(|(i, _)| i)
            .collect();

        if res_ids.is_empty() || work_ids.is_empty() {
            return;
        }

        for _ in 0..count {
            let home = res_ids[self.rng.range(res_ids.len())];
            let work = work_ids[self.rng.range(work_ids.len())];
            let id = self.citizens.len() as u32;

            self.citizens.push(Citizen {
                id,
                home_b: home,
                work_b: work,
                current_b: home,
                state: CitizenState::Dwelling,
                dwell_left: self.rng.between(2.0, 25.0),
                has_car: self.rng.f() > 0.25,
            });
        }
    }

    pub fn update_citizens(&mut self, dt: f64) {
        let n_citizens = self.citizens.len();
        if n_citizens == 0 {
            return;
        }

        for i in 0..n_citizens {
            if self.citizens[i].state == CitizenState::Dwelling {
                if self.citizens[i].dwell_left > 0.0 {
                    self.citizens[i].dwell_left -= dt;
                    continue;
                }

                if self.trip_scale < 0.99 && self.rng.f() > self.trip_scale {
                    self.citizens[i].dwell_left = 2.0;
                    continue;
                }

                let cur_b = self.citizens[i].current_b;
                let home_b = self.citizens[i].home_b;
                let work_b = self.citizens[i].work_b;

                let target_b = if cur_b == home_b {
                    if self.rng.f() > 0.25 {
                        work_b
                    } else {
                        self.rng.range(self.buildings.len())
                    }
                } else {
                    home_b
                };

                let dwell_base = self.rng.between(8.0, 22.0) / (self.spawn_rate * 0.25).max(1.0);
                if target_b == cur_b {
                    self.citizens[i].dwell_left = dwell_base * self.dwell_scale;
                    continue;
                }

                let (bx0, by0) = (self.buildings[cur_b].x, self.buildings[cur_b].y);
                let (bx1, by1) = (self.buildings[target_b].x, self.buildings[target_b].y);
                let dist = (bx1 - bx0).hypot(by1 - by0);

                let has_car = self.citizens[i].has_car && self.buildings[cur_b].cars_parked > 0;

                if dist < 150.0 || !has_car {
                    if !self.start_walk_trip(i, target_b) {
                        self.citizens[i].dwell_left = 3.0;
                    }
                } else {
                    if !self.start_car_trip(i, target_b) {
                        if self.rng.f() > 0.5 {
                            if !self.start_walk_trip(i, target_b) {
                                self.citizens[i].dwell_left = 3.0;
                            }
                        } else {
                            self.citizens[i].dwell_left = 2.5;
                        }
                    }
                }
            }
        }
    }
}
