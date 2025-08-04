#![no_std]
#![no_main]

use async_button::{Button, ButtonConfig, ButtonEvent};
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_time::{Duration, Ticker};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_wifi::esp_now::{PeerInfo, BROADCAST_ADDRESS};
use esp_wifi::{init, EspWifiController};
use spark_messages::{ButtonNumber, Message, MessageType, PROTOCOL_VERSION};

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    //info!("Embassy initialized!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let esp_wifi_ctrl = &*mk_static!(
        EspWifiController<'static>,
        init(timg0.timer0, Rng::new(peripherals.RNG)).unwrap()
    );

    let wifi = peripherals.WIFI;
    let (mut controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, wifi).unwrap();
    controller.set_mode(esp_wifi::wifi::WifiMode::Sta).unwrap();
    controller.start().unwrap();

    let mut esp_now = interfaces.esp_now;
    esp_now.set_channel(11).unwrap();

    println!("esp-now version {}", esp_now.version().unwrap());

    let c = InputConfig::default().with_pull(Pull::Up);

    let mut async_button = Button::new(Input::new(peripherals.GPIO21, c), ButtonConfig::default());
    let mut async_button2 = Button::new(Input::new(peripherals.GPIO0, c), ButtonConfig::default());
    let mut async_button3 = Button::new(Input::new(peripherals.GPIO14, c), ButtonConfig::default());
    let mut async_button4 = Button::new(Input::new(peripherals.GPIO35, c), ButtonConfig::default());

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

    loop {
        let event1 = async_button.update();
        let event2 = async_button2.update();
        let event3 = async_button3.update();
        let event4 = async_button4.update();

        let event_data: (ButtonNumber, ButtonEvent);
        match embassy_futures::select::select4(event1, event2, event3, event4).await {
            embassy_futures::select::Either4::First(e) => {
                println!("button1: {:?}", e);
                event_data = (ButtonNumber::Button1, e);
            }
            embassy_futures::select::Either4::Second(e) => {
                println!("button2: {:?}", e);
                event_data = (ButtonNumber::Button2, e);
            }
            embassy_futures::select::Either4::Third(e) => {
                println!("button3: {:?}", e);
                event_data = (ButtonNumber::Button3, e);
            }
            embassy_futures::select::Either4::Fourth(e) => {
                println!("button4: {:?}", e);
                event_data = (ButtonNumber::Button4, e);
            }
        }

        let mut tx_bux: [u8; 32] = [0; 32];
        let message = Message {
            protocol_version: PROTOCOL_VERSION,
            message_type: MessageType::ButtonEvent {
                button_number: event_data.0,
                event_type: event_data.1.into(),
            },
        };

        postcard::to_slice(&message, &mut tx_bux).unwrap();

        esp_now
            .send_async(&BROADCAST_ADDRESS, &tx_bux)
            .await
            .unwrap();
    }
}
