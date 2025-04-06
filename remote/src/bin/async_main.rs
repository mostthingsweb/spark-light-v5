#![no_std]
#![no_main]

use core::cell::RefCell;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Event, Input, Io, Pull};
use esp_hal::handler;
use esp_hal::interrupt::InterruptConfigurable;
use esp_println::{print, println};
use log::info;

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    // TODO: Spawn some tasks
    let _ = spawner;

    let mut button = Input::new(peripherals.GPIO21, Pull::Up);
    let mut button2 = Input::new(peripherals.GPIO0, Pull::Up);
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(handler);

    critical_section::with(|cs| {
        // Here we are listening for a low level to demonstrate
        // that you need to stop listening for level interrupts,
        // but usually you'd probably use `FallingEdge`.
        button.listen(Event::FallingEdge);
        button2.listen(Event::FallingEdge);
        BUTTON.borrow_ref_mut(cs).replace(button);
        BUTTON2.borrow_ref_mut(cs).replace(button2);
    });

    loop {
        println!("WAT");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/v0.23.1/examples/src/bin
}

// You will need to store the `Input` object in a static variable so
// that the interrupt handler can access it.
static BUTTON: Mutex<RefCell<Option<Input>>> =
    Mutex::new(RefCell::new(None));
static BUTTON2: Mutex<RefCell<Option<Input>>> =
    Mutex::new(RefCell::new(None));
#[handler]
fn handler() {
    critical_section::with(|cs| {
        let mut button = BUTTON.borrow_ref_mut(cs);
        let Some(button) = button.as_mut() else {
            // Some other interrupt has occurred
            // before the button was set up.
            return;
        };

        if button.is_interrupt_set() {
            println!("Button pressed");

            // If you want to stop listening for interrupts, you need to
            // call `unlisten` here. If you comment this line, the
            // interrupt will fire continuously while the button
            // is pressed.
            //button.unlisten();
        }

        let mut button2 = BUTTON2.borrow_ref_mut(cs);
        let Some(button2) = button2.as_mut() else {
            // Some other interrupt has occurred
            // before the button was set up.
            return;
        };

        if button2.is_interrupt_set() {
            println!("Button2 pressed");

            // If you want to stop listening for interrupts, you need to
            // call `unlisten` here. If you comment this line, the
            // interrupt will fire continuously while the button
            // is pressed.
            //button.unlisten();
        }
    });
}