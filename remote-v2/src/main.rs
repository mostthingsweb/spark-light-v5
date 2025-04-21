use std::{
    collections::HashMap,
    num::NonZero,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use esp_idf_svc::{hal::{
    cpu::core, delay::BLOCK, gpio::{AnyIOPin, AnyInputPin, IOPin, Input, InputPin, InterruptType, Level, Pin, PinDriver}, peripheral::Peripheral, prelude::Peripherals, task::notification::Notification, timer::{self, Timer, TimerDriver}
}, timer::EspTimerService};

struct ButtonControlBlock<'a> {
    pin_driver: PinDriver<'a, AnyInputPin, Input>,
    last_state: Level,
    last_state_change: Duration,
}

fn bits_set(n: usize) -> u32 {
    if n == 0 {
        0
    } else {
        u32::MAX >> (32 - n)
    }
}

fn run<'d, TIMER: Timer>(sender: Sender<u32>, buttons: Vec<AnyInputPin>, mut timer_instance: impl Peripheral<P = TIMER> + 'd,) {
    println!("Starting control_led() on core {:?}", core());

    let timer_service = EspTimerService::new().unwrap();

    let mut buttons: HashMap<_, ButtonControlBlock> = buttons
        .into_iter()
        .enumerate()
        .map(|(i, pin)| {
            let pin_driver = PinDriver::input(pin).unwrap();
            let level = pin_driver.get_level();
            (
                i,
                ButtonControlBlock {
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

    let waker = notification.notifier();
    let bit = bits_set(buttons.len());
    let timer = timer_service.timer(move || { 
        unsafe {
            waker.notify(NonZero::new(bit).unwrap());
        }
    }).unwrap();

    timer.every(Duration::from_millis(100)).unwrap();

    loop {
        for (i, button) in buttons.iter_mut() {
            let waker = notification.notifier();
            let bit = 1 << i;

            unsafe {
                button
                    .pin_driver
                    .subscribe(move || {
                        waker.notify(NonZero::new(bit).unwrap());
                    })
                    .unwrap();
            }

            button.pin_driver.enable_interrupt().unwrap();
        }

        // block until notified
        loop {
            if let Some(notification_value) = notification.wait(BLOCK) {
                let notification_value: u32 = notification_value.into();
                //println!("{:?}", timer_service.now());

                for (i, button) in buttons.iter_mut() {
                    if notification_value & (1 << i) != 0 {
                        let new_level = button.pin_driver.get_level();
                        if new_level != button.last_state {
                            println!("\tbutton{}: {}", i, bool::from(new_level));
                            // TODO: Send the pin #
                            sender.send(*i as u32).unwrap();
                            button.last_state = new_level;
                        } else if button.last_state == Level::Low {
                            let elapsed = timer_service.now();
                            if elapsed - button.last_state_change > Duration::from_millis(500) {
                                println!("\tbutton{}: LONG PRESS?", i);
                            }
                        }          
                    }
                }

                break;
            }
        }
    }
}

fn i2c_loop(receiver: Receiver<u32>) {
    loop {
        if let Ok(b) = receiver.recv_timeout(Duration::from_secs(1)) {
            println!("s: {}", b);
        } else {
            println!("timeout");
        }
    }
    // let mut rx_buf: [u8; 8] = [0; 8];
    // let config = I2cSlaveConfig::new()
    //     .rx_buffer_length(SLAVE_BUFFER_SIZE)
    //     .tx_buffer_length(SLAVE_BUFFER_SIZE);
    // let driver = I2cSlaveDriver::new(Peripherals::, sda, scl, slave_addr, &config)?;
}

const SLAVE_ADDR: u8 = 0x22;
const SLAVE_BUFFER_SIZE: usize = 128;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let (tx, rx) = mpsc::channel::<u32>();

    let peripherals = Peripherals::take().unwrap();

    let buttons = vec![
        peripherals.pins.gpio21.downgrade_input(),
        peripherals.pins.gpio0.downgrade_input(),
        peripherals.pins.gpio14.downgrade_input(),
        peripherals.pins.gpio35.downgrade_input(),
    ];

    std::thread::scope(|s| {
        s.spawn(|| {
            run(tx, buttons, peripherals.timer00);
        });

        s.spawn(|| {
            i2c_loop(rx);
        });
    });

    Ok(())
}
