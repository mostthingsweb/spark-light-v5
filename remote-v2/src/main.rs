use std::{default, time::Duration};

use async_button::{Button, ButtonConfig, ButtonEvent};
use embassy_executor::Executor;
use esp_idf_svc::hal::{cpu::{self, core}, gpio::{Gpio0, Gpio14, Gpio21, Gpio35, Input, InputPin, OutputPin, PinDriver}, i2c::{I2c, I2cSlaveConfig, I2cSlaveDriver}, peripheral::Peripheral, prelude::Peripherals, task::block_on};
use futures_util::{select, FutureExt};
use static_cell::StaticCell;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn run(p0: Gpio0, p21: Gpio21, p14: Gpio14, p35: Gpio35) {
    println!("Starting control_led() on core {:?}", core());

    let mut async_button = Button::new(
        PinDriver::input(p21).unwrap(),
        ButtonConfig::default(),
    );
    let mut async_button2 = Button::new(
        PinDriver::input(p0).unwrap(),
        ButtonConfig::default(),
    );
    let mut async_button3 = Button::new(
        PinDriver::input(p14).unwrap(),
        ButtonConfig::default(),
    );
    let mut async_button4 = Button::new(
        PinDriver::input(p35).unwrap(),
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

fn i2c_loop<'d, I2C: I2c>(i2c: impl Peripheral<P = I2C> + 'd, sda: impl Peripheral<P = impl InputPin + OutputPin> + 'd, scl: impl Peripheral<P = impl InputPin + OutputPin> + 'd) -> anyhow::Result<()> {
    println!("Starting i2c_loop() on core {:?}", core());

    let mut rx_buf: [u8; 8] = [0; 8];
    let config = I2cSlaveConfig::new()
        .rx_buffer_length(SLAVE_BUFFER_SIZE)
        .tx_buffer_length(SLAVE_BUFFER_SIZE);
    let driver = I2cSlaveDriver::new(i2c, sda, scl, SLAVE_ADDR, &config)?;
    
    loop {
        std::thread::sleep(Duration::from_secs(2));
        println!("s");
    }
    
    Ok(())
}

const SLAVE_ADDR: u8 = 0x22;
const SLAVE_BUFFER_SIZE: usize = 128;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    ThreadSpawnConfiguration {
        pin_to_core: Some(cpu::Core::Core0),
        ..Default::default()
    }.set().unwrap();

    let thread0 = std::thread::Builder::new()
    .stack_size(7000)
    .spawn(move || {
        let executor = EXECUTOR.init(Executor::new());
        executor.run(|spawner| {
            spawner.spawn(run(peripherals.pins.gpio0, peripherals.pins.gpio21, peripherals.pins.gpio14, peripherals.pins.gpio35)).unwrap();
        });
    })?;

    ThreadSpawnConfiguration {
        pin_to_core: Some(cpu::Core::Core1),
        ..Default::default()
    }.set().unwrap();

    let thread1 = std::thread::Builder::new()
    .stack_size(7000)
    .spawn(move || { 
        i2c_loop(peripherals.i2c0, peripherals.pins.gpio1, peripherals.pins.gpio2).unwrap();
    })?;

    //thread0.join().unwrap();
    thread1.join().unwrap();

    Ok(())
}
