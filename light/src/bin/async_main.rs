#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{AnyPin, Input, Io, Output, OutputPin, Pull};
use esp_hal::peripheral::Peripheral;
use esp_hal::rmt::{Channel, Rmt};
use esp_hal::time::RateExtU32;
use esp_hal::Blocking;
use log::info;

use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};

extern crate alloc;

#[embassy_executor::task]
async fn light_task(
    mut led1: SmartLedsAdapter<Channel<Blocking, 0>, 193>,
    mut led2: SmartLedsAdapter<Channel<Blocking, 1>, 193>,
    mut led3: SmartLedsAdapter<Channel<Blocking, 2>, 193>,
    mut led4: SmartLedsAdapter<Channel<Blocking, 3>, 193>,
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
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let freq = 80.MHz();
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();
    let rmt_buffer = smartLedBuffer!(8);

    let led1 = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO35, rmt_buffer);
    let led2 = SmartLedsAdapter::new(rmt.channel1, peripherals.GPIO36, rmt_buffer);
    let led3 = SmartLedsAdapter::new(rmt.channel2, peripherals.GPIO38, rmt_buffer);
    let led4 = SmartLedsAdapter::new(rmt.channel3, peripherals.GPIO37, rmt_buffer);

    spawner.spawn(light_task(led1, led2, led3, led4)).unwrap();

    let mut io = Io::new(peripherals.IO_MUX);

    let mut irq_pin = Input::new(peripherals.GPIO4, Pull::Up);

}
