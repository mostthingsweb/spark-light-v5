use esp_idf_svc::espnow::{EspNow, PeerInfo, BROADCAST};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::{FreeRtos, BLOCK};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi, WifiDeviceId};
use postcard::from_bytes;
use spark_messages::Test;
use std::thread;
use std::thread::yield_now;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(7000)
            .spawn_scoped(s, || -> anyhow::Result<()> {
                let peripherals = Peripherals::take().unwrap();
                let config = I2cConfig::new().baudrate(100_u32.kHz().into());
                let mut driver = I2cDriver::new(
                    peripherals.i2c0,
                    peripherals.pins.gpio4,
                    peripherals.pins.gpio16,
                    &config,
                )
                .unwrap();

                let tx_buf: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
                let mut rx_buf: [u8; 32] = [0; 32];
                driver.write(0x23, &tx_buf, BLOCK)?;
                driver.read(0x23, &mut rx_buf, BLOCK)?;

                println!("{:?}", &rx_buf);

                let decoded: Test = from_bytes(&rx_buf)?;
                dbg!(decoded);

                let sys_loop = EspSystemEventLoop::take()?;
                let nvs = EspDefaultNvsPartition::take()?;

                let mut wifi = BlockingWifi::wrap(
                    EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
                    sys_loop,
                )?;

                let mac = wifi.wifi().get_mac(WifiDeviceId::Sta)?;
                println!("{:x?}", mac);

                let mac = wifi.wifi().get_mac(WifiDeviceId::Ap)?;
                println!("{:x?}", mac);

                let conf = Configuration::Client(ClientConfiguration::default());
                wifi.set_configuration(&conf).unwrap();
                wifi.start().unwrap();

                let espnow: EspNow<'_> = EspNow::take()?;
                espnow.register_recv_cb(|d, a| {
                    dbg!(d, a);
                })?;

                loop {
                    FreeRtos::delay_ms(100);
                }

                Ok(())
            })
            .unwrap();
    });

    loop {
        thread::sleep(Duration::from_secs(5));
    }

    log::info!("Hello, world!");
    Ok(())
}
