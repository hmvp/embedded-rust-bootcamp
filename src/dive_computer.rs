use core::{fmt, ops::Div};

use num::FromPrimitive;

const MAX_DEPTH: u32 = 40_000;
const MAX_SAFE_ASCEND_RATE: i32 = -15;
const MAX_AIR: u32 = 2000 * 100;
const AIR_INCREMENT: u32 = 500;

#[derive(PartialEq, Clone, Copy)]
pub enum Unit {
    Metric,
    Imperial,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self {
            Unit::Imperial => "FT",
            Unit::Metric => "M",
        };

        // Write to buffer
        write!(f, "{}", unit)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Alarm {
    High,
    Medium,
    Low,
    None,
}

impl Alarm {
    pub fn display_len(&self) -> usize {
        match self {
            Alarm::High => 4,
            Alarm::Medium => 6,
            Alarm::Low => 3,
            Alarm::None => 4,
        }
    }
}

impl fmt::Display for Alarm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alarm = match self {
            Alarm::High => "HIGH",
            Alarm::Medium => "MEDIUM",
            Alarm::Low => "LOW",
            Alarm::None => "NONE",
        };

        // Write to buffer
        writeln!(f, "{:13}", alarm)
    }
}

// #[derive(Clone, Copy)]
pub struct DiveComputer {
    unit: Unit,
    depth: u32,
    rate: i32,
    air: u32,
    edt: u64,
}

impl DiveComputer {
    pub fn new() -> Self {
        DiveComputer {
            unit: Unit::Metric,
            air: 5000,
            depth: 0,
            edt: 0,
            rate: 0,
        }
    }

    fn get_alarm(&self) -> Alarm {
        if gas_to_surface_in_cl(self.depth / 1000) > self.air {
            return Alarm::High;
        }

        if self.rate < MAX_SAFE_ASCEND_RATE {
            return Alarm::Medium;
        }

        if self.depth > MAX_DEPTH {
            return Alarm::Low;
        }

        Alarm::None
    }

    pub fn fill_air(&mut self) {
        if self.depth == 0 {
            self.air += AIR_INCREMENT;
            if self.air > MAX_AIR {
                self.air = MAX_AIR;
            }
        }
    }

    pub fn increase_rate(&mut self) {
        self.rate += 1;
        if self.rate > 50 {
            self.rate = 50
        }
    }

    pub fn decrease_rate(&mut self) {
        self.rate -= 1;
        if self.rate < -50 {
            self.rate = -50
        }
    }

    pub fn change_depth(&mut self, interval: u32) {
        // Change depth based on rate

        let hz = 1000 / interval;
        let rate_in_mm_per_interval = self.rate * 1000 / (60 * hz as i32);

        self.depth = ((self.depth as i32) + rate_in_mm_per_interval).clamp(0, i32::MAX) as u32;

        if self.depth == 0 {
            if self.rate < 0 {
                // Reset rate since we can't ascend out of the water
                self.rate = 0;
            }
        } else {
            // Underwater stuff
            self.edt = self.edt + interval as u64;
            self.air = self.air.saturating_sub(gas_rate_in_cl(self.depth / 1000) / hz);
        }
    }

    pub fn toggle_unit(&mut self) {
        self.unit = if self.unit == Unit::Imperial { Unit::Metric } else { Unit::Imperial }
    }
}

impl Default for DiveComputer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DiveComputer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let depth = if self.unit == Unit::Imperial { mm2ft(self.depth) } else { self.depth / 1000 };
        let rate = if self.unit == Unit::Imperial { mm2ft(self.rate * 1000) } else { self.rate };

        let hours = self.edt / 3600000;
        let minutes = self.edt % 3600000 / 60000;
        let seconds = self.edt % 3600000 % 60000 / 1000;

        // Write to buffer
        writeln!(f, "DiveMaster")?;
        writeln!(f)?;
        writeln!(f, "DEPTH: {:width$}{}", depth, self.unit, width = if self.unit == Unit::Imperial { 11 } else { 12 })?;
        writeln!(f, "RATE: {:width$}{}/M", rate, self.unit, width = if self.unit == Unit::Imperial { 10 } else { 11 })?;
        writeln!(f, "AIR: {:14}L", self.air / 100)?;
        writeln!(f, "EDT: {:9}:{:0>2}:{:0>2}", hours, minutes, seconds)?;
        writeln!(f, "ALARM: {:width$}{}", "", self.get_alarm(), width = 13 - self.get_alarm().display_len())
    }
}

const RESPIRATORY_MINUTE_VOLUME_CL: u32 = 1200;
const RESPIRATORY_SECOND_VOLUME_CL: u32 = RESPIRATORY_MINUTE_VOLUME_CL / 60;

/// Calculate gas rate in centi liter for a depth in meters
///
/// # Examples
///
/// ```
/// assert_eq!(gas_rate_in_cl(0), 20);
/// assert_eq!(gas_rate_in_cl(10), 40);
/// ```
///
fn gas_rate_in_cl(depth_in_m: u32) -> u32 {
    /* 10m of water = 1 bar = 100 centibar */
    let ambient_pressure_in_cb = 100 + (10 * depth_in_m);

    /* Gas consumed at STP = RSV * ambient pressure / standard pressure */
    (RESPIRATORY_SECOND_VOLUME_CL * ambient_pressure_in_cb) / 100
}

fn gas_to_surface_in_cl(depth_in_m: u32) -> u32 {
    let mut gas = 0;
    let secs_to_ascend_1m = 60 / (-MAX_SAFE_ASCEND_RATE) as u32;

    for depth in 0..depth_in_m {
        gas += gas_rate_in_cl(depth) * secs_to_ascend_1m;
    }

    gas
}

fn mm2ft<T: Div<Output = T> + FromPrimitive>(depth: T) -> T {
    depth / FromPrimitive::from_u32(305).unwrap()
}

#[cfg(test)]
mod test {

    use super::*;
    use defmt::assert;

    #[test]
    fn test_gas_rate_in_cl() {
        assert!(true)
    }
}
