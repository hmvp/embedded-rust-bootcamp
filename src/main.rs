#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;

use arraystring::{typenum::U200, ArrayString};
use cortex_m::{asm::nop, interrupt::Mutex};
use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
use embedded_time::{duration::Extensions, fixed_point::FixedPoint};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
use pimoroni_pico_explorer as bsp;

use bsp::{PicoExplorer, Screen, XOSC_CRYSTAL_FREQ};

use bsp::hal::{
    adc::Adc,
    clocks::{init_clocks_and_plls, Clock},
    gpio::{self, Interrupt::EdgeLow},
    pac::{self, interrupt},
    pwm,
    sio::Sio,
    timer::{Alarm, Alarm0, Alarm1},
    watchdog::Watchdog,
    Timer,
};

mod dive_computer;
use dive_computer::*;

const TIME_TICK_MS: u32 = 500;

type APin = gpio::Pin<gpio::bank0::Gpio12, gpio::PullUpInput>;
type BPin = gpio::Pin<gpio::bank0::Gpio13, gpio::PullUpInput>;
type XPin = gpio::Pin<gpio::bank0::Gpio14, gpio::PullUpInput>;
type YPin = gpio::Pin<gpio::bank0::Gpio15, gpio::PullUpInput>;
type LEDPin = gpio::Pin<gpio::bank0::Gpio25, gpio::Output<gpio::PushPull>>;

type Buttons = (APin, BPin, XPin, YPin);
type LedScreenAlarm = (LEDPin, Screen, Alarm1);

static GLOBAL_DIVE_COMPUTER: Mutex<RefCell<Option<DiveComputer>>> = Mutex::new(RefCell::new(None));
static GLOBAL_BUTTONS: Mutex<RefCell<Option<Buttons>>> = Mutex::new(RefCell::new(None));
static GLOBAL_LED_SCREEN_ALARM: Mutex<RefCell<Option<LedScreenAlarm>>> = Mutex::new(RefCell::new(None));
static GLOBAL_DIVE_TICK_ALARM: Mutex<RefCell<Option<Alarm0>>> = Mutex::new(RefCell::new(None));

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

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut alarm0 = timer.alarm_0().unwrap();
    alarm0.enable_interrupt();
    let _ = alarm0.schedule(10.microseconds());
    let mut alarm1 = timer.alarm_1().unwrap();
    alarm1.enable_interrupt();
    let _ = alarm1.schedule(10.microseconds());
    // Enable adc
    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let (explorer, pins) = PicoExplorer::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, pac.SPI0, adc, &mut pac.RESETS, &mut delay);

    explorer.a.set_interrupt_enabled(EdgeLow, true);
    explorer.b.set_interrupt_enabled(EdgeLow, true);
    explorer.x.set_interrupt_enabled(EdgeLow, true);
    explorer.y.set_interrupt_enabled(EdgeLow, true);

    let led = pins.led.into_push_pull_output();

    let dive_computer = DiveComputer::default();

    // Store for use in interrupts
    cortex_m::interrupt::free(|cs| {
        GLOBAL_BUTTONS.borrow(cs).replace(Some((explorer.a, explorer.b, explorer.x, explorer.y)));
        GLOBAL_DIVE_COMPUTER.borrow(cs).replace(Some(dive_computer));
        GLOBAL_LED_SCREEN_ALARM.borrow(cs).replace(Some((led, explorer.screen, alarm1)));
        GLOBAL_DIVE_TICK_ALARM.borrow(cs).replace(Some(alarm0));

        // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
        // will jump to the interrupt function when the interrupt occurs.
        // We do this last so that the interrupt can't go off while
        // it is in the middle of being configured
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
        }
    });

    loop {
        nop()
    }
}

#[interrupt]
fn TIMER_IRQ_1() {
    // Create a fixed buffer to store screen contents
    static mut BUF: Option<ArrayString<U200>> = None;
    // The `#[interrupt]` attribute covertly converts this to `&'static mut Option<Buttons>`
    static mut LED_SCREEN_ALARM: Option<LedScreenAlarm> = None;

    // This is one-time lazy initialisation. We steal the variables given to us
    // via `LED`.
    if LED_SCREEN_ALARM.is_none() {
        cortex_m::interrupt::free(|cs| {
            *LED_SCREEN_ALARM = GLOBAL_LED_SCREEN_ALARM.borrow(cs).take();
        });
    }

    if BUF.is_none() {
        *BUF = Some(ArrayString::<U200>::new());
    }

    if let Some(((led, screen, alarm0), buf)) = LED_SCREEN_ALARM.as_mut().zip(*BUF).as_mut() {
        alarm0.clear_interrupt();
        let _ = alarm0.schedule(TIME_TICK_MS.microseconds() * 500);

        if led.is_set_low().unwrap() {
            info!("on!");
            led.set_high().unwrap();
        } else {
            info!("off!");
            led.set_low().unwrap();
        }

        buf.clear();

        cortex_m::interrupt::free(|cs| {
            let mut d_ref = GLOBAL_DIVE_COMPUTER.borrow(cs).borrow_mut();
            let dive_computer = d_ref.as_mut().unwrap();

            // Write to buffer
            writeln!(buf, "{}", dive_computer).unwrap();
        });

        // Draw buffer on screen
        let style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(Rgb565::GREEN)
            .background_color(Rgb565::BLACK)
            .build();
        Text::with_alignment(buf, Point::new(20, 30), style, Alignment::Left).draw(screen).unwrap();
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    // The `#[interrupt]` attribute covertly converts this to `&'static mut Option<Buttons>`
    static mut DIVE_TICK_ALARM: Option<Alarm0> = None;

    // This is one-time lazy initialisation. We steal the variables given to us
    // via `LED`.
    if DIVE_TICK_ALARM.is_none() {
        cortex_m::interrupt::free(|cs| {
            *DIVE_TICK_ALARM = GLOBAL_DIVE_TICK_ALARM.borrow(cs).take();
        });
    }

    if let Some(alarm0) = DIVE_TICK_ALARM {
        alarm0.clear_interrupt();
        let _ = alarm0.schedule(TIME_TICK_MS.microseconds() * 1000);

        cortex_m::interrupt::free(|cs| {
            GLOBAL_DIVE_COMPUTER.borrow(cs).borrow_mut().as_mut().unwrap().change_depth(TIME_TICK_MS);
        });
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    // The `#[interrupt]` attribute covertly converts this to `&'static mut Option<Buttons>`
    static mut BUTTONS: Option<Buttons> = None;

    // This is one-time lazy initialisation. We steal the variables given to us
    // via `BUTTONS`.
    if BUTTONS.is_none() {
        cortex_m::interrupt::free(|cs| {
            *BUTTONS = GLOBAL_BUTTONS.borrow(cs).take();
        });
    }

    if let Some((a, b, x, y)) = BUTTONS {
        if a.interrupt_status(EdgeLow) {
            cortex_m::interrupt::free(|cs| {
                GLOBAL_DIVE_COMPUTER.borrow(cs).borrow_mut().as_mut().unwrap().fill_air();
            });
            a.clear_interrupt(EdgeLow);
        }

        // Change unit
        if b.interrupt_status(EdgeLow) {
            cortex_m::interrupt::free(|cs| {
                GLOBAL_DIVE_COMPUTER.borrow(cs).borrow_mut().as_mut().unwrap().toggle_unit();
            });
            b.clear_interrupt(EdgeLow);
        }

        // Increase descend
        if x.interrupt_status(EdgeLow) {
            cortex_m::interrupt::free(|cs| {
                GLOBAL_DIVE_COMPUTER.borrow(cs).borrow_mut().as_mut().unwrap().increase_rate();
            });
            x.clear_interrupt(EdgeLow);
        }

        // Increase ascend
        if y.interrupt_status(EdgeLow) {
            cortex_m::interrupt::free(|cs| {
                GLOBAL_DIVE_COMPUTER.borrow(cs).borrow_mut().as_mut().unwrap().decrease_rate();
            });
            y.clear_interrupt(EdgeLow);
        }
    }
}
