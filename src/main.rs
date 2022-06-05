//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::{
    fmt::{self, Display, Write},
    ops::Div,
    write,
};
use cortex_m_rt::entry;

use arraystring::{typenum::U200, ArrayString};
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use embedded_hal::digital::v2::OutputPin;
use embedded_time::{duration::Milliseconds, fixed_point::FixedPoint};
use num::FromPrimitive;
use panic_probe as _;
// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use pimoroni_pico_explorer as bsp;

use bsp::{Button, PicoExplorer, XOSC_CRYSTAL_FREQ};

use bsp::hal::{
    adc::Adc,
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

const MAX_DEPTH: u32 = 40_000;
const MAX_SAFE_ASCEND_RATE: i32 = -15;
const MAX_AIR: u32 = 2000 * 100;
const AIR_INCREMENT: u32 = 500;
const TIME_TICK_MS: u32 = 50;

#[derive(PartialEq)]
enum Unit {
    Metric,
    Imperial,
}

impl Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self {
            Unit::Imperial => "FT",
            Unit::Metric => "M",
        };

        // Write to buffer
        write!(f, "{}", unit)
    }
}

#[derive(Debug)]
enum Alarm {
    High,
    Medium,
    Low,
    None,
}
impl Display for Alarm {
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
struct DiveComputer {
    unit: Unit,
    depth: u32,
    rate: i32,
    air: u32,
    edt: Milliseconds,
}

impl DiveComputer {
    fn get_alarm(&self) -> Alarm {
        if gas_to_surface_in_cl(self.depth) > self.air {
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
}

impl Display for DiveComputer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let depth = if self.unit == Unit::Imperial {
            mm2ft(self.depth)
        } else {
            self.depth / 1000
        };
        let rate = if self.unit == Unit::Imperial {
            mm2ft(self.rate * 1000)
        } else {
            self.rate
        };

        let hours = self.edt / 3600000;
        let minutes = self.edt.integer() % 3600000 / 60000;
        let seconds = self.edt.integer() % 3600000 % 60000 / 1000;

        // Write to buffer
        writeln!(f, "DiveMaster")?;
        writeln!(f, "")?;
        writeln!(
            f,
            "DEPTH: {:width$}{}",
            depth,
            self.unit,
            width = if self.unit == Unit::Imperial { 11 } else { 12 }
        )?;
        writeln!(
            f,
            "RATE: {:width$}{}/M",
            rate,
            self.unit,
            width = if self.unit == Unit::Imperial { 10 } else { 11 }
        )?;
        writeln!(f, "AIR: {:14}L", self.air / 100)?;
        writeln!(f, "EDT: {:10}:{:0>2}:{:0>2}", hours, minutes, seconds)?;
        writeln!(f, "ALARM: {:13}", self.get_alarm())
    }
}

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Enable watchdog and clocks
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // Enable adc
    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let (mut explorer, pins) = PicoExplorer::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        pac.SPI0,
        adc,
        &mut pac.RESETS,
        &mut delay,
    );

    // Create a fixed buffer to store screen contents
    let mut buf = ArrayString::<U200>::new();

    let mut dive_computer = DiveComputer {
        unit: Unit::Metric,
        air: 5000,
        depth: 0,
        edt: Milliseconds::new(0),
        rate: 0,
    };

    let mut led = pins.led.into_push_pull_output();

    let mut counter = 0;

    loop {
        if counter == 0 {
            info!("on!");
            led.set_high().unwrap();
        } else if counter == 500 {
            info!("off!");
            led.set_low().unwrap();
        }

        let logic_tick = if counter == 0 || counter == 500 {
            true
        } else {
            false
        };

        if dive_computer.depth > 0 {
            // Underwater stuff
            dive_computer.edt = dive_computer.edt + Milliseconds::new(TIME_TICK_MS);
            if logic_tick {
                dive_computer.air = dive_computer
                    .air
                    .saturating_sub(gas_rate_in_cl(dive_computer.depth) * 2);
            }
        } else {
            // Fill air
            if explorer.is_pressed(Button::A) {
                dive_computer.air += AIR_INCREMENT;
                if dive_computer.air > MAX_AIR {
                    dive_computer.air = MAX_AIR;
                }
            }

            // Reset rate since we can't ascend
            dive_computer.rate = 0;
        }

        // Change unit
        if explorer.is_pressed(Button::B) {
            dive_computer.unit = if dive_computer.unit == Unit::Imperial {
                Unit::Metric
            } else {
                Unit::Imperial
            }
        }

        // Increase descend
        if explorer.is_pressed(Button::X) {
            dive_computer.rate += 1;
            if dive_computer.rate > 50 {
                dive_computer.rate = 50
            }
        }

        // Increase ascend
        if explorer.is_pressed(Button::Y) {
            dive_computer.rate -= 1;
            if dive_computer.rate < -50 {
                dive_computer.rate = -50
            }
        }

        if logic_tick {
            // Change depth based on rate
            dive_computer.depth = ((dive_computer.depth as i32) + (dive_computer.rate * 1000 / 120))
                .clamp(0, i32::MAX) as u32;
        }

        // Write to buffer
        buf.clear();
        writeln!(&mut buf, "{}", dive_computer).unwrap();

        // Draw buffer on screen
        let style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(Rgb565::GREEN)
            .background_color(Rgb565::BLACK)
            .build();
        Text::with_alignment(&buf, Point::new(20, 30), style, Alignment::Left)
            .draw(&mut explorer.screen)
            .unwrap();

        counter += TIME_TICK_MS;
        if counter >= 1000 {
            counter = 0;
        }
        delay.delay_ms(TIME_TICK_MS);
    }
}

const RMV: u32 = 1200;
const RHSV: u32 = RMV / 120;

fn gas_rate_in_cl(depth_in_mm: u32) -> u32 {
    let depth_in_m = depth_in_mm / 1000;

    /* 10m of water = 1 bar = 100 centibar */
    let ambient_pressure_in_cb = 100 + (10 * depth_in_m);

    /* Gas consumed at STP = RHSV * ambient pressure / standard pressure */
    (RHSV * ambient_pressure_in_cb) / 100
}

fn gas_to_surface_in_cl(depth_in_mm: u32) -> u32 {
    let mut gas = 0;
    let halfsecs_to_ascend_1m = (2 * 60) / (-MAX_SAFE_ASCEND_RATE) as u32;
    let depth_in_m = depth_in_mm / 1000;

    for depth in 0..depth_in_m {
        gas += gas_rate_in_cl(depth * 1000) * halfsecs_to_ascend_1m;
    }

    gas
}

fn mm2ft<T: Div<Output = T> + FromPrimitive>(depth: T) -> T {
    depth / FromPrimitive::from_u32(305).unwrap()
}
