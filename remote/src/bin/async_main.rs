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
    let mut async_button3 = Button::new(Input::new(peripherals.GPIO14, Pull::Up), ButtonConfig::default());
    let mut async_button4 = Button::new(Input::new(peripherals.GPIO35, Pull::Up), ButtonConfig::default());

    loop {
        let event1 = async_button.update();
        let event2 = async_button2.update();
        let event3 = async_button3.update();
        let event4 = async_button4.update();

        match embassy_futures::select::select4(event1, event2, event3, event4).await {
            embassy_futures::select::Either4::First(e) => {
                println!("button1: {:?}", e);
            }
            embassy_futures::select::Either4::Second(e) => {
                println!("button2: {:?}", e);
            }
            embassy_futures::select::Either4::Third(e) => {
                println!("button3: {:?}", e);
            }
            embassy_futures::select::Either4::Fourth(e) => {
                println!("button4: {:?}", e);
            }
        }
    }
}
