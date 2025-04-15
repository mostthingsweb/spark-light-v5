use std::time::Duration;

use async_button::{Button, ButtonConfig, ButtonEvent};
use embassy_executor::Executor;
use esp_idf_svc::hal::{cpu::{self, core}, gpio::PinDriver, i2c::I2cSlaveConfig, prelude::Peripherals, task::block_on};
use futures_util::{select, FutureExt};
use static_cell::StaticCell;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn run() {
    println!("Starting control_led() on core {:?}", core());

    let peripherals = Peripherals::take().unwrap();

    let mut async_button = Button::new(
        PinDriver::input(peripherals.pins.gpio21).unwrap(),
        ButtonConfig::default(),
    );
    let mut async_button2 = Button::new(
        PinDriver::input(peripherals.pins.gpio0).unwrap(),
        ButtonConfig::default(),
    );
    let mut async_button3 = Button::new(
        PinDriver::input(peripherals.pins.gpio14).unwrap(),
        ButtonConfig::default(),
    );
    let mut async_button4 = Button::new(
        PinDriver::input(peripherals.pins.gpio35).unwrap(),
        ButtonConfig::default(),
    );

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

fn i2c_loop() {
    loop {
        std::thread::sleep(Duration::from_secs(2));
        println!("s");
    }
    // let mut rx_buf: [u8; 8] = [0; 8];
    // let config = I2cSlaveConfig::new()
    //     .rx_buffer_length(SLAVE_BUFFER_SIZE)
    //     .tx_buffer_length(SLAVE_BUFFER_SIZE);
    // let driver = I2cSlaveDriver::new(Peripherals::, sda, scl, slave_addr, &config)?;
    
}

const SLAVE_ADDR: u8 = 0x22;
const SLAVE_BUFFER_SIZE: usize = 128;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let thread0 = std::thread::Builder::new()
    .stack_size(7000)
    .spawn(move || {
        let executor = EXECUTOR.init(Executor::new());
        executor.run(|spawner| {
            spawner.spawn(run()).unwrap();
        });
    })?;

    let thread1 = std::thread::Builder::new()
    .stack_size(7000)
    .spawn(move || { 
        i2c_loop();
    })?;

    thread0.join().unwrap();
    thread1.join().unwrap();

    Ok(())
}
