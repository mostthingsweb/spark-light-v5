#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use core::cell::RefCell;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::{handler, ram, Blocking};
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use esp_hal::rmt::{Channel, ConstChannelAccess, Rmt, Tx};
use esp_hal::rng::Rng;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::{
    EspWifiController,
    esp_now::{BROADCAST_ADDRESS, PeerInfo},
    init,
};
use esp_println::println;
use embassy_futures::select::{Either, select};
use esp_wifi::esp_now::{EspNowManager, EspNowReceiver};
use esp_backtrace as _;
use esp_hal::gpio::{Event, Input, InputConfig, Io};

static REMOTE_MAC: [u8; 6] = [0xC8, 0xF0, 0x9E, 0x2C, 0x28, 0x8C];

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

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[embassy_executor::task]
async fn listener(manager: &'static EspNowManager<'static>, mut receiver: EspNowReceiver<'static>) {
    loop {
        let r = receiver.receive_async().await;
        println!("Received {:?}", r.data());
        if r.info.dst_address == BROADCAST_ADDRESS {
            if !manager.peer_exists(&r.info.src_address) {
                manager
                    .add_peer(PeerInfo {
                        interface: esp_wifi::esp_now::EspNowWifiInterface::Sta,
                        peer_address: r.info.src_address,
                        lmk: None,
                        channel: None,
                        encrypt: false,
                    })
                    .unwrap();
                println!("Added peer {:?}", r.info.src_address);
            }
        }
    }
}

#[embassy_executor::task]
async fn interrupt_listen() {

}

static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

#[handler]
#[ram]
fn handler() {
    println!(
        "GPIO Interrupt with priority {}",
        esp_hal::xtensa_lx::interrupt::get_level()
    );

    if critical_section::with(|cs| {
        BUTTON
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .is_interrupt_set()
    }) {
        println!("Button was the source of the interrupt");
    } else {
        println!("Button was not the source of the interrupt");
    }

    critical_section::with(|cs| {
        BUTTON
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt()
    });
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    //
    // let mut io = Io::new(peripherals.IO_MUX);
    // io.set_interrupt_handler(handler);
    //
    // let config = InputConfig::default();
    // let mut button = Input::new(peripherals.GPIO4, config);
    //
    // critical_section::with(|cs| {
    //     button.listen(Event::RisingEdge);
    //     BUTTON.borrow_ref_mut(cs).replace(button)
    // });

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

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let esp_wifi_ctrl = &*mk_static!(EspWifiController<'static>,         init(timg0.timer0, Rng::new(peripherals.RNG)).unwrap());

    let wifi = peripherals.WIFI;
    let (mut controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, wifi).unwrap();
    controller.set_mode(esp_wifi::wifi::WifiMode::Sta).unwrap();
    controller.start().unwrap();

    let mut esp_now = interfaces.esp_now;
    esp_now.set_channel(11).unwrap();

    println!("esp-now version {}", esp_now.version().unwrap());

    println!("{:?}", REMOTE_MAC);
    if !esp_now.peer_exists(&BROADCAST_ADDRESS) {
        esp_now
            .add_peer(PeerInfo {
                interface: esp_wifi::esp_now::EspNowWifiInterface::Sta,
                peer_address: BROADCAST_ADDRESS,
                lmk: None,
                channel: None,
                encrypt: false,
            })
            .unwrap();
    }

    let (manager, sender, receiver) = esp_now.split();
    let manager = mk_static!(EspNowManager<'static>, manager);
    spawner.spawn(listener(manager, receiver)).ok();
    spawner.spawn(light_task(led1, led2, led3, led4)).unwrap();

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}
