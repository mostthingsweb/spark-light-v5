use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::delay::BLOCK;
use postcard::from_bytes;
use spark_messages::Test;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let config = I2cConfig::new().baudrate(100_u32.kHz().into());
    let mut driver = I2cDriver::new(peripherals.i2c0, peripherals.pins.gpio4, peripherals.pins.gpio16, &config)?;

    let tx_buf: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
    let mut rx_buf: [u8; 32] = [0; 32];
    driver.write(0x23, &tx_buf, BLOCK)?;
    driver.read(0x23, &mut rx_buf, BLOCK)?;

    dbg!(rx_buf);    

    let decoded: Test = from_bytes(&rx_buf)?;
    dbg!(decoded);

    log::info!("Hello, world!");
    Ok(())
}
