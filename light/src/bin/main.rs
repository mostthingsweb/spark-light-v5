#![no_std]
#![no_main]

use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::main;
use esp_hal::rmt::Rmt;
use esp_hal::time::RateExtU32;
use esp_hal::timer::timg::TimerGroup;
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use smart_leds::{brightness, gamma, hsv::{hsv2rgb, Hsv}, SmartLedsWrite, RGB8};

extern crate alloc;

use core::panic::PanicInfo;

// Define the panic handler for ESP32 (or other embedded environments)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {} // Infinite loop to halt execution
}

#[main]
fn main() -> ! {
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_println::logger::init_logger_from_env();

    esp_alloc::heap_allocator!(72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timg0.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let freq = 80.MHz();
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();

    let rmt_buffer = smartLedBuffer!(8);

    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO35, rmt_buffer);
    let mut led2 = SmartLedsAdapter::new(rmt.channel1, peripherals.GPIO36, rmt_buffer);
    let mut led3 = SmartLedsAdapter::new(rmt.channel2, peripherals.GPIO38, rmt_buffer);
    let mut led4 = SmartLedsAdapter::new(rmt.channel3, peripherals.GPIO37, rmt_buffer);

    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;

    let delay = Delay::new();
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
                brightness(gamma(data.iter().cloned()), 10).next().unwrap()
            ];

             led.write(data2.iter().cloned())
                 .unwrap();
             led2.write(data2.iter().cloned())
                 .unwrap();
            led3.write(data2.iter().cloned())
                 .unwrap();
            led4.write(data2.iter().cloned())
                .unwrap();
            delay.delay_millis(20);
        }
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/v0.23.1/examples/src/bin
}
