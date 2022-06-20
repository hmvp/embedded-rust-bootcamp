#![deny(unsafe_code)]
#![deny(warnings)]
#![cfg(not(test))]
#![no_std]
#![no_main]
use core::fmt::Write;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use arraystring::{typenum::U200, ArrayString};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
use embedded_time::fixed_point::FixedPoint;
use fugit::MicrosDurationU32;

// Provide an alias for our BSP so we can switch targets quickly.
use pimoroni_pico_explorer as bsp;

use bsp::{Button, PicoExplorer, XOSC_CRYSTAL_FREQ};

use bsp::hal::{
    adc::Adc,
    clocks::{init_clocks_and_plls, Clock},
    entry, pac,
    sio::Sio,
    watchdog::Watchdog,
};

use dive_computer::DiveComputer;

const TIME_TICK_MS: u32 = 50;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Enable watchdog and clocks
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(XOSC_CRYSTAL_FREQ, pac.XOSC, pac.CLOCKS, pac.PLL_SYS, pac.PLL_USB, &mut pac.RESETS, &mut watchdog)
        .ok()
        .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // Enable adc
    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let (mut explorer, pins) = PicoExplorer::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, pac.SPI0, adc, &mut pac.RESETS, &mut delay);

    // Create a fixed buffer to store screen contents
    let mut buf = ArrayString::<U200>::new();

    let mut led = pins.led.into_push_pull_output();

    let mut dive_computer = DiveComputer::default();

    let mut counter = 0;

    loop {
        if led.is_set_low().unwrap() {
            info!("on!");
            led.set_high().unwrap();
        } else {
            info!("off!");
            led.set_low().unwrap();
        }

        // Fill air
        if explorer.is_pressed(Button::A) {
            dive_computer.fill_air();
        }

        // Change unit
        if explorer.is_pressed(Button::B) {
            dive_computer.toggle_unit();
        }

        // Increase descend
        if explorer.is_pressed(Button::X) {
            dive_computer.increase_rate();
        }

        // Increase ascend
        if explorer.is_pressed(Button::Y) {
            dive_computer.decrease_rate();
        }

        if counter == 0 {
            // Change depth based on rate
            dive_computer.change_depth(MicrosDurationU32::millis(500))
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
        if counter >= 500 {
            counter = 0;
        }
        delay.delay_ms(TIME_TICK_MS);
    }
}
