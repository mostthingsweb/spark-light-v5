use std::{
    collections::HashMap,
    num::NonZero,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use esp_idf_svc::espnow::{PeerInfo, BROADCAST};
use esp_idf_svc::sys::{esp_base_mac_addr_get, esp_err_t, esp_now_peer_info_t, EspError, ESP_ERR_EFUSE, ESP_ERR_TIMEOUT};
use esp_idf_svc::wifi::{ClientConfiguration, Configuration, WifiDeviceId};
use esp_idf_svc::{
    espnow::EspNow,
    eventloop::EspSystemEventLoop,
    hal::{
        delay::BLOCK,
        gpio::{AnyIOPin, AnyInputPin, IOPin, Input, InputPin, InterruptType, Level, PinDriver},
        i2c::{I2c, I2cSlaveConfig, I2cSlaveDriver},
        modem::{Modem, WifiModemPeripheral},
        peripheral::Peripheral,
        prelude::Peripherals,
        task::{self, notification::Notification, yield_now},
        timer::Timer,
        units::Hertz,
    },
    nvs::EspDefaultNvsPartition,
    sys::{sleep, DR_REG_GPIO_BASE},
    timer::EspTimerService,
    wifi::{BlockingWifi, EspWifi},
};
use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_svc::io::ErrorKind::TimedOut;
use postcard::to_slice;
use spark_messages::{Button, ButtonSequence, Test};

struct ButtonControlBlock<'a> {
    button: Button,
    pin_driver: PinDriver<'a, AnyInputPin, Input>,
    last_state: Level,

    #[allow(unused)]
    last_state_change: Duration,
}

// fn bits_set(n: usize) -> u32 {
//     if n == 0 {
//         0
//     } else {
//         u32::MAX >> (32 - n)
//     }
// }

fn monitor_buttons_task<'d, TIMER: Timer>(
    sender: Sender<Button>,
    buttons: HashMap<Button, AnyInputPin>,
    #[allow(unused)] mut timer_instance: impl Peripheral<P = TIMER> + 'd,
) {
    println!(
        "Starting monitor_buttons_task(), task {:?}",
        task::current().unwrap()
    );

    let timer_service = EspTimerService::new().unwrap();

    let mut buttons: HashMap<_, ButtonControlBlock> = buttons
        .into_iter()
        .enumerate()
        .map(|(i, (button, pin))| {
            let pin_driver = PinDriver::input(pin).unwrap();
            let level = pin_driver.get_level();
            (
                i,
                ButtonControlBlock {
                    button,
                    pin_driver,
                    last_state: level,
                    last_state_change: timer_service.now(),
                },
            )
        })
        .collect();

    for button in buttons.values_mut() {
        button
            .pin_driver
            .set_interrupt_type(InterruptType::AnyEdge)
            .unwrap();
    }

    let notification = Notification::new();

    // let waker = notification.notifier();
    // let bit = bits_set(buttons.len());
    // let timer = timer_service.timer(move || {
    //     unsafe {
    //         waker.notify(NonZero::new(bit).unwrap());
    //     }
    // }).unwrap();

    // timer.every(Duration::from_millis(100)).unwrap();

    loop {
        for (i, button) in buttons.iter_mut() {
            let waker = notification.notifier();
            let bit = 1 << i;

            unsafe {
                button
                    .pin_driver
                    .subscribe(move || {
                        waker.notify_and_yield(NonZero::new(bit).unwrap());
                    })
                    .unwrap();
            }

            button.pin_driver.enable_interrupt().unwrap();
        }

        // block until notified
        loop {
            if let Some(notification_value) = notification.wait(TickType::new_millis(50).into()) {
                let notification_value: u32 = notification_value.into();
                //println!("{:?}", timer_service.now());

                for (i, button) in buttons.iter_mut() {
                    if notification_value & (1 << i) != 0 {
                        let new_level = button.pin_driver.get_level();
                        if new_level != button.last_state {
                            //println!("\tbutton{}: {}", i, bool::from(new_level));
                            button.last_state = new_level;

                            if new_level == Level::Low {
                                sender.send(button.button).unwrap();
                            }
                        }
                        // else if button.last_state == Level::Low {
                        //     let elapsed = timer_service.now();
                        //     if elapsed - button.last_state_change > Duration::from_millis(500) {
                        //         println!("\tbutton{}: LONG PRESS?", i);
                        //     }
                        // }
                    }
                }

                // Break out of loop so we can re-arm interrupts
                break;
            } else {
                //task::do_yield();
            }
        }
    }
}

fn button_sequence_debounce_task(receiver: Receiver<Button>, sender: Sender<smallvec::SmallVec<[Button; 5]>>) {
    println!(
        "Starting button_sequence_debounce_task(), task {:?}",
        task::current().unwrap()
    );

    let timer_service = EspTimerService::new().unwrap();
    let mut button_sequence: smallvec::SmallVec<[Button; 5]> = smallvec::SmallVec::new();
    let mut last_button_press: Option<Duration> = None;
    loop {
        let mut got_button = false;
        if let Ok(b) = receiver.recv_timeout(Duration::from_millis(50)) {
            button_sequence.push(b);
            got_button = true;
        }

        let now = timer_service.now();

        // Send no matter what if 5 or more buttons are queued up
        let mut should_send = button_sequence.len() >= 5;

        // Send if 2 seconds since last press has elapsed
        if !button_sequence.is_empty() {
            if let Some(last_button_press) = &last_button_press {
                if now - *last_button_press >= Duration::from_millis(700) {
                    should_send = true;
                }
            }
        }

        if got_button {
            last_button_press = Some(now);
        }

        if should_send {
            println!("{:?}", button_sequence);
            sender.send(button_sequence).unwrap();
            button_sequence = smallvec::SmallVec::new();
            last_button_press = None;
        }

        //task::do_yield();
    }
}

fn esp_now_task<'d, MODEM: WifiModemPeripheral>(
    receiver: Receiver<smallvec::SmallVec<[Button; 5]>>,
    modem: impl Peripheral<P = MODEM> + 'd,
) -> anyhow::Result<()> {
    println!(
        "Starting esp_now_task(), task {:?}",
        task::current().unwrap()
    );

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;

    let mac = wifi.wifi().get_mac(WifiDeviceId::Sta).unwrap();
    println!("{:x?}", mac);

    let mac = wifi.wifi().get_mac(WifiDeviceId::Ap).unwrap();
    println!("{:x?}", mac);

    let conf = Configuration::Client(ClientConfiguration::default());
    wifi.set_configuration(&conf).unwrap();
    wifi.start().unwrap();

    let espnow: EspNow<'_> = EspNow::take().unwrap();
    let peer = PeerInfo {
        peer_addr: BROADCAST,
        channel: 0,
        ..Default::default()
    };

    espnow.add_peer(peer).unwrap();

    loop {
        let ret = receiver.recv_timeout(Duration::from_millis(10));
        if ret.is_ok() {
            let mut buf: [u8; 32] = [0; 32];
            postcard::to_slice(&ButtonSequence { buttons: ret.unwrap()}, &mut buf).unwrap();
            println!("sending broadcast: {:?}", buf);
            espnow.send(BROADCAST, &buf).unwrap();
        } else {
            //task::do_yield();
        }
    }
}

const SLAVE_ADDR: u8 = 0x23;
const SLAVE_BUFFER_SIZE: usize = 128;

fn i2c_task<'d, M: I2c>(
    i2c: impl Peripheral<P = M> + 'd,
    sda: AnyIOPin,
    scl: AnyIOPin,
) -> anyhow::Result<()> {
    let mut rx_buf: [u8; 8] = [0; 8];
    let config = I2cSlaveConfig::new()
        .rx_buffer_length(SLAVE_BUFFER_SIZE)
        .tx_buffer_length(SLAVE_BUFFER_SIZE);
    let mut driver = I2cSlaveDriver::new(i2c, sda, scl, SLAVE_ADDR, &config)?;

    let d = Test {
        wat: 10,
        version: 1.234,
    };

    let mut tx_buf: [u8; 32] = [0; 32];
    to_slice(&d, &mut tx_buf)?;

    loop {
        println!("WAITING FOR COMMAND");
        let mut rx_buf: [u8; 8] = [0; 8];
        match driver.read(&mut rx_buf, TickType::new_millis(100).into()) {
            Ok(_) => {
                driver.write(&tx_buf, TickType::new_millis(100).into()).unwrap();
                println!("Slave receives {:?}", rx_buf);
            }
            Err(e) => {
                if e.code() != ESP_ERR_TIMEOUT {
                     println!("Error: {:?}", e);
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let (tx, rx) = mpsc::channel::<Button>();
    let (tx_button_seq, rx_button_seq) = mpsc::channel::<smallvec::SmallVec<[Button; 5]>>();

    let peripherals = Peripherals::take().unwrap();

    let buttons = maplit::hashmap! {
        Button::Button0 => peripherals.pins.gpio21.downgrade_input(),
        Button::Button1 => peripherals.pins.gpio0.downgrade_input(),
        Button::Button2 => peripherals.pins.gpio14.downgrade_input(),
        Button::Button3 => peripherals.pins.gpio35.downgrade_input(),
    };

    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(7000)
            .spawn_scoped(s, || {
                monitor_buttons_task(tx, buttons, peripherals.timer00);
            })
            .unwrap();

        s.spawn(|| {
            button_sequence_debounce_task(rx, tx_button_seq);
        });

        std::thread::Builder::new()
            .stack_size(7000)
            .spawn_scoped(s, || {
            esp_now_task(rx_button_seq, peripherals.modem).unwrap();
        }).unwrap();

        s.spawn(|| {
            i2c_task(
                peripherals.i2c0,
                peripherals.pins.gpio4.downgrade(),
                peripherals.pins.gpio16.downgrade(),
            )
        });
    });

    Ok(())
}
