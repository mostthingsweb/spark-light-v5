#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::Blocking;
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use esp_hal::rmt::{Channel, ConstChannelAccess, Rmt, Tx};
use esp_hal::time::Rate;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn light_task(
    mut led1: SmartLedsAdapter<ConstChannelAccess<Tx, 0>, 193>,
    mut led2: SmartLedsAdapter<ConstChannelAccess<Tx, 1>, 193>,
    mut led3: SmartLedsAdapter<ConstChannelAccess<Tx, 2>, 193>,
    mut led4: SmartLedsAdapter<ConstChannelAccess<Tx, 3>, 193>,
) {
    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;

    loop {
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [hsv2rgb(color)];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.

            let data2: &[RGB8; 8] = &[
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
                brightness(gamma(data.iter().cloned()), 10).next().unwrap(),
            ];

            led1.write(data2.iter().cloned()).unwrap();
            led2.write(data2.iter().cloned()).unwrap();
            led3.write(data2.iter().cloned()).unwrap();
            led4.write(data2.iter().cloned()).unwrap();
            Timer::after(Duration::from_millis(20)).await;
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    let freq = Rate::from_mhz(80);
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();
    let rmt_buffer = smart_led_buffer!(8);

    let led1 = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO35, rmt_buffer);
    let led2 = SmartLedsAdapter::new(rmt.channel1, peripherals.GPIO36, rmt_buffer);
    let led3 = SmartLedsAdapter::new(rmt.channel2, peripherals.GPIO38, rmt_buffer);
    let led4 = SmartLedsAdapter::new(rmt.channel3, peripherals.GPIO37, rmt_buffer);

    spawner.spawn(light_task(led1, led2, led3, led4)).unwrap();
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}
