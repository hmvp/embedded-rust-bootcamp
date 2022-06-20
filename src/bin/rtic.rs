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
use fugit::{MicrosDurationU32, MicrosDurationU64};
use rp2040_monotonic::Rp2040Monotonic;
use rtic::Monotonic;

// Provide an alias for our BSP so we can switch targets quickly.
use pimoroni_pico_explorer as bsp;

use bsp::{PicoExplorer, Screen, XOSC_CRYSTAL_FREQ};

use bsp::hal::{
    adc::Adc,
    clocks::{init_clocks_and_plls, Clock},
    gpio::{self, Interrupt::EdgeLow, Interrupt::LevelLow},
    sio::{self, Sio},
    watchdog::Watchdog,
};

use dive_computer::DiveComputer;

const REPEAT_TIME: MicrosDurationU64 = MicrosDurationU64::millis(200);
const DEBOUNCE_TIME: MicrosDurationU64 = MicrosDurationU64::millis(100);
const UI_TASK_INTERVAL: MicrosDurationU64 = MicrosDurationU64::millis(100);
const LOGIC_TICK_INTERVAL: MicrosDurationU64 = MicrosDurationU64::millis(500);

type APin = gpio::Pin<gpio::bank0::Gpio12, gpio::PullUpInput>;
type BPin = gpio::Pin<gpio::bank0::Gpio13, gpio::PullUpInput>;
type XPin = gpio::Pin<gpio::bank0::Gpio14, gpio::PullUpInput>;
type YPin = gpio::Pin<gpio::bank0::Gpio15, gpio::PullUpInput>;
type LEDPin = gpio::Pin<gpio::bank0::Gpio25, gpio::Output<gpio::PushPull>>;

type Instant = <Rp2040Monotonic as Monotonic>::Instant;

#[rtic::app(device = bsp::hal::pac, peripherals = true, dispatchers = [TIMER_IRQ_1, TIMER_IRQ_2])]
mod app {

    use super::*;

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
        let mut pac = cx.device;
        let mut core = cx.core;

        // Soft-reset does not release the hardware spinlocks
        // Release them now to avoid a deadlock after debug or watchdog reset
        unsafe {
            sio::spinlock_reset();
        }

        // Enable watchdog and clocks
        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let clocks = init_clocks_and_plls(XOSC_CRYSTAL_FREQ, pac.XOSC, pac.CLOCKS, pac.PLL_SYS, pac.PLL_USB, &mut pac.RESETS, &mut watchdog)
            .ok()
            .unwrap();

        let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

        // Enable adc
        let adc = Adc::new(pac.ADC, &mut pac.RESETS);

        let sio = Sio::new(pac.SIO);

        let mono = Rp2040Monotonic::new(pac.TIMER);

        let (explorer, pins) = PicoExplorer::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, pac.SPI0, adc, &mut pac.RESETS, &mut delay);

        explorer.a.set_interrupt_enabled(EdgeLow, true);
        explorer.b.set_interrupt_enabled(EdgeLow, true);
        explorer.x.set_interrupt_enabled(EdgeLow, true);
        explorer.y.set_interrupt_enabled(EdgeLow, true);

        explorer.a.set_interrupt_enabled(LevelLow, true);
        explorer.b.set_interrupt_enabled(LevelLow, true);
        explorer.x.set_interrupt_enabled(LevelLow, true);
        explorer.y.set_interrupt_enabled(LevelLow, true);

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
    fn ui_output(mut cx: ui_output::Context, interval: MicrosDurationU64) {
        ui_output::spawn_after(interval, interval).unwrap();

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
    fn dive_tick(mut cx: dive_tick::Context, interval: MicrosDurationU64) {
        dive_tick::spawn_after(interval, interval).unwrap();

        cx.shared.dive_computer.lock(|dive_computer| {
            dive_computer.change_depth(MicrosDurationU32::try_from(interval).unwrap());
        });
    }

    #[task(binds = IO_IRQ_BANK0, shared = [dive_computer], local = [button_a, button_b, button_x, button_y, last_triggered: Instant = Instant::from_ticks(0)])]
    fn button_handler(mut cx: button_handler::Context) {
        let trigger_time = monotonics::now();

        let time_waited = if trigger_time <= *cx.local.last_triggered {
            MicrosDurationU64::micros(0)
        } else {
            trigger_time - *cx.local.last_triggered
        };

        let mut triggered = false;

        macro_rules! handle_button {
            ($button:tt, $func:ident) => {
                if cx.local.$button.interrupt_status(EdgeLow) {
                    if time_waited > DEBOUNCE_TIME {
                        cx.shared.dive_computer.lock(|dive_computer| {
                            dive_computer.$func();
                        });
                        triggered = true;
                    }
                    cx.local.$button.clear_interrupt(EdgeLow);
                } else if cx.local.$button.interrupt_status(LevelLow) {
                    if time_waited > REPEAT_TIME {
                        cx.shared.dive_computer.lock(|dive_computer| {
                            dive_computer.$func();
                        });
                        triggered = true
                    }
                    cx.local.$button.clear_interrupt(LevelLow);
                }
            };
        }

        // Fill air
        handle_button!(button_a, fill_air);

        // Change unit
        handle_button!(button_b, toggle_unit);

        // Increase descend
        handle_button!(button_x, increase_rate);

        // Increase ascend
        handle_button!(button_y, decrease_rate);

        if triggered {
            info!("button pushed");
            *cx.local.last_triggered = trigger_time;
        }
    }
}
