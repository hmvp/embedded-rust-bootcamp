//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::{
    fmt::{self, Display, Write},
    write,
};
use cortex_m_rt::entry;

use arraystring::{typenum::U100, ArrayString};
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use embedded_hal::digital::v2::OutputPin;
use embedded_time::{duration::Seconds, fixed_point::FixedPoint};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use pimoroni_pico_explorer as bsp;

use bsp::{PicoExplorer, XOSC_CRYSTAL_FREQ};

use bsp::hal::{
    adc::Adc,
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

const MAX_DEPTH: u32 = 40_000;

enum Unit {
    Metric,
    Imperial,
}

impl Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self {
            Unit::Imperial => "ft",
            Unit::Metric => "m",
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

struct DiveComputer {
    unit: Unit,
    depth: u32,
    rate: u32,
    air: u32,
    edt: Seconds,
}

impl DiveComputer {
    fn get_alarm(&self) -> Alarm {
        // if

        if self.depth > MAX_DEPTH {
            return Alarm::Low;
        }

        Alarm::None
    }
}

impl Display for DiveComputer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write to buffer
        writeln!(f, "DiveMaster")?;
        writeln!(f, "DEPTH: {}{}", self.depth, self.unit)?;
        writeln!(f, "RATE: {}{}", self.rate, self.unit)?;
        writeln!(f, "AIR: {}L", self.air)?;
        writeln!(f, "EDT: {}", self.edt)?;
        writeln!(f, "ALARM: {:?}", self.get_alarm())
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
    let mut buf = ArrayString::<U100>::new();

    let dive_computer = DiveComputer {
        unit: Unit::Metric,
        air: 1,
        depth: 3,
        edt: Seconds::new(3),
        rate: 6,
    };

    let mut led = pins.led.into_push_pull_output();

    loop {
        info!("on!");
        led.set_high().unwrap();
        delay.delay_ms(500);
        info!("off!");
        led.set_low().unwrap();
        delay.delay_ms(500);

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
    }
}
