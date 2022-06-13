#![deny(unsafe_code)]
// #![deny(warnings)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use arraystring::{typenum::U200, ArrayString};
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
use embedded_time::fixed_point::FixedPoint;
use panic_probe as _;
// Provide an alias for our BSP so we can switch targets quickly.
use pimoroni_pico_explorer as bsp;

use bsp::{PicoExplorer, Screen, XOSC_CRYSTAL_FREQ};

use bsp::hal::{
    adc::Adc,
    clocks::{init_clocks_and_plls, Clock},
    gpio::{self, Interrupt::EdgeLow, Interrupt::LevelLow},
    pac::{self},
    sio::Sio,
    watchdog::Watchdog,
};
use core::fmt::Write;
use rp2040_monotonic::{fugit::ExtU64, Rp2040Monotonic};
mod dive_computer;
use dive_computer::DiveComputer;

type APin = gpio::Pin<gpio::bank0::Gpio12, gpio::PullUpInput>;
type BPin = gpio::Pin<gpio::bank0::Gpio13, gpio::PullUpInput>;
type XPin = gpio::Pin<gpio::bank0::Gpio14, gpio::PullUpInput>;
type YPin = gpio::Pin<gpio::bank0::Gpio15, gpio::PullUpInput>;
type LEDPin = gpio::Pin<gpio::bank0::Gpio25, gpio::Output<gpio::PushPull>>;

#[rtic::app(device = bsp::hal::pac, peripherals = true, dispatchers = [TIMER_IRQ_1, TIMER_IRQ_2])]
mod app {

    use rp2040_monotonic::fugit::{MillisDurationU64, TimerInstantU64};

    use super::*;

    const REPEAT_TIME: u64 = 200;
    const UI_TASK_INTERVAL: u64 = 100;
    const LOGIC_TICK_INTERVAL: u64 = 500;

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type Mono = Rp2040Monotonic;

    // Resources shared between tasks
    #[shared]
    struct Shared {
        dive_computer: DiveComputer,
    }

    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        screen: Screen,
        led: LEDPin,
        buffer: ArrayString<U200>,
        button_a: APin,
        button_b: BPin,
        button_x: XPin,
        button_y: YPin,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("Program start");
        let mut pac: pac::Peripherals = cx.device;
        let mut core = cx.core;

        // Enable watchdog and clocks
        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let clocks = init_clocks_and_plls(XOSC_CRYSTAL_FREQ, pac.XOSC, pac.CLOCKS, pac.PLL_SYS, pac.PLL_USB, &mut pac.RESETS, &mut watchdog)
            .ok()
            .unwrap();
        let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

        let mono = Rp2040Monotonic::new(pac.TIMER);

        let adc = Adc::new(pac.ADC, &mut pac.RESETS);
        let sio = Sio::new(pac.SIO);

        let (explorer, pins) = PicoExplorer::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, pac.SPI0, adc, &mut pac.RESETS, &mut delay);

        explorer.a.set_interrupt_enabled(EdgeLow, true);
        explorer.b.set_interrupt_enabled(EdgeLow, true);
        explorer.y.set_interrupt_enabled(EdgeLow, true);
        explorer.x.set_interrupt_enabled(EdgeLow, true);
        explorer.a.set_interrupt_enabled(LevelLow, true);
        explorer.b.set_interrupt_enabled(LevelLow, true);
        explorer.y.set_interrupt_enabled(LevelLow, true);
        explorer.x.set_interrupt_enabled(LevelLow, true);

        ui_output::spawn(UI_TASK_INTERVAL).unwrap();
        dive_tick::spawn(LOGIC_TICK_INTERVAL).unwrap();

        // Set the ARM SLEEPONEXIT bit to go to sleep after handling interrupts
        // See https://developer.arm.com/docs/100737/0100/power-management/sleep-mode/sleep-on-exit-bit
        core.SCB.set_sleepdeep();

        (
            // Initialization of shared resources
            Shared {
                dive_computer: DiveComputer::default(),
            },
            // Initialization of task local resources
            Local {
                screen: explorer.screen,
                led: pins.led.into_push_pull_output(),
                buffer: ArrayString::<U200>::new(),
                button_a: explorer.a,
                button_b: explorer.b,
                button_x: explorer.x,
                button_y: explorer.y,
            },
            // Move the monotonic timer to the RTIC run-time, this enables
            // scheduling
            init::Monotonics(mono),
        )
    }

    // Background task, runs whenever no other tasks are running
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            // Now Wait For Interrupt is used instead of a busy-wait loop
            // to allow MCU to sleep between interrupts
            // https://developer.arm.com/documentation/ddi0406/c/Application-Level-Architecture/Instruction-Details/Alphabetical-list-of-instructions/WFI
            rtic::export::wfi();
        }
    }

    #[task(shared = [dive_computer], local = [screen, led, buffer], priority = 2)]
    fn ui_output(mut cx: ui_output::Context, interval: u64) {
        ui_output::spawn_after(interval.millis(), interval).unwrap();

        let ui_output::LocalResources { screen, led, buffer } = cx.local;

        if led.is_set_low().unwrap() {
            info!("on!");
            led.set_high().unwrap();
        } else {
            info!("off!");
            led.set_low().unwrap();
        }

        buffer.clear();

        cx.shared.dive_computer.lock(|dive_computer| {
            // Write to buffer
            writeln!(buffer, "{}", dive_computer).unwrap();
        });

        // Draw buffer on screen
        let style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(Rgb565::GREEN)
            .background_color(Rgb565::BLACK)
            .build();
        Text::with_alignment(buffer, Point::new(20, 30), style, Alignment::Left).draw(screen).unwrap();
    }

    #[task(shared = [dive_computer], local = [], priority = 2)]
    fn dive_tick(mut cx: dive_tick::Context, interval: u64) {
        dive_tick::spawn_after(interval.millis(), interval).unwrap();

        cx.shared.dive_computer.lock(|dive_computer| {
            dive_computer.change_depth(interval as u32);
        });
    }

    #[task(binds = IO_IRQ_BANK0, shared = [dive_computer], local = [button_a, button_b, button_x, button_y, last_triggered: u64 = 0])]
    fn button_handler(mut cx: button_handler::Context) {
        let trigger_time = monotonics::now();
        let wait = trigger_time - TimerInstantU64::<1_000_000>::from_ticks(*cx.local.last_triggered) < MillisDurationU64::millis(REPEAT_TIME);

        if !wait {
            info!("button pushed");
        }

        // Fill air
        handle_button!(cx, trigger_time, wait, button_a, fill_air);

        // Change unit
        handle_button!(cx, trigger_time, wait, button_b, toggle_unit);

        // Increase descend
        handle_button!(cx, trigger_time, wait, button_x, increase_rate);

        // Increase ascend
        handle_button!(cx, trigger_time, wait, button_y, decrease_rate);
    }

    macro_rules! handle_button {
        ($cx:ident, $time:ident, $wait:ident, $button:tt, $func:ident) => {
            if $cx.local.$button.interrupt_status(EdgeLow) {
                if !$wait {
                    $cx.shared.dive_computer.lock(|dive_computer| {
                        dive_computer.$func();
                    });
                    *$cx.local.last_triggered = $time.ticks();
                }
                $cx.local.$button.clear_interrupt(EdgeLow);
            }
            if $cx.local.$button.interrupt_status(LevelLow) {
                if !$wait {
                    $cx.shared.dive_computer.lock(|dive_computer| {
                        dive_computer.$func();
                    });
                    *$cx.local.last_triggered = ($time + MillisDurationU64::millis(REPEAT_TIME)).ticks();
                }
                $cx.local.$button.clear_interrupt(LevelLow);
            }
        };
    }
}
