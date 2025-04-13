#![no_std]
#![no_main]

use core::cell::RefCell;
use async_button::{Button, ButtonConfig};
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

    let mut async_button = Button::new(Input::new(peripherals.GPIO21, Pull::Up), ButtonConfig::default());
    let mut async_button2 = Button::new(Input::new(peripherals.GPIO0, Pull::Up), ButtonConfig::default());

    loop {
        let event1 = async_button.update();
        let event2 = async_button2.update();

        match embassy_futures::select::select(event1, event2).await {
            embassy_futures::select::Either::First(e) => {
                println!("button1: {:?}", e);
            },
            embassy_futures::select::Either::Second(e) => {
                println!("button2: {:?}", e);
            },
        }
    }
}

// // You will need to store the `Input` object in a static variable so
// // that the interrupt handler can access it.
// static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
// static BUTTON2: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

// #[handler]
// fn handler() {
//     critical_section::with(|cs| {
//         let mut button = BUTTON.borrow_ref_mut(cs);
//         let Some(button) = button.as_mut() else {
//             // Some other interrupt has occurred
//             // before the button was set up.
//             return;
//         };

//         if button.is_interrupt_set() {
//             println!("Button pressed");

//             // If you want to stop listening for interrupts, you need to
//             // call `unlisten` here. If you comment this line, the
//             // interrupt will fire continuously while the button
//             // is pressed.
//             //button.unlisten();
//         }

//         let mut button2 = BUTTON2.borrow_ref_mut(cs);
//         let Some(button2) = button2.as_mut() else {
//             // Some other interrupt has occurred
//             // before the button was set up.
//             return;
//         };

//         if button2.is_interrupt_set() {
//             println!("Button2 pressed");

//             // If you want to stop listening for interrupts, you need to
//             // call `unlisten` here. If you comment this line, the
//             // interrupt will fire continuously while the button
//             // is pressed.
//             //button.unlisten();
//         }
//     });
// }
