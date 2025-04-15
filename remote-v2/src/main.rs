use async_button::{Button, ButtonConfig, ButtonEvent};
use embassy_executor::Executor;
use esp_idf_svc::hal::{gpio::{PinDriver}, prelude::Peripherals, task::block_on};
use futures_util::{select, FutureExt};
use static_cell::StaticCell;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();


#[embassy_executor::task]
async fn run() {
    
    let peripherals = Peripherals::take().unwrap();

    let mut pin0 = PinDriver::input(peripherals.pins.gpio21).unwrap();
    let mut pin1 = PinDriver::input(peripherals.pins.gpio0).unwrap();
    let pin2 = PinDriver::input(peripherals.pins.gpio14).unwrap();
    let mut button = Button::new(pin2, ButtonConfig::default());

    log::info!("Hello, world!");

    loop {
        let mut a = Box::pin(pin0.wait_for_falling_edge().fuse());
        let mut b = Box::pin(pin1.wait_for_falling_edge().fuse());
        let mut c = Box::pin(button.update().fuse());

        select! {
            _ = a => log::info!("button 0"),
            _ = b => log::info!("button 1"),
            c_res = c => { 
                log::info!("button 2: {:?}", c_res);
            },
            complete => continue,
        };
    }
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(run()).unwrap();
    });

    Ok(())
}
