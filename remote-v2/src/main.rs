use std::{
    collections::HashMap,
    num::NonZero,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use esp_idf_svc::hal::{
    cpu::core,
    delay::BLOCK,
    gpio::{AnyIOPin, AnyInputPin, IOPin, InputPin, InterruptType, Pin, PinDriver},
    prelude::Peripherals,
    task::notification::Notification,
};

fn run(sender: Sender<u32>, buttons: Vec<AnyInputPin>) {
    println!("Starting control_led() on core {:?}", core());

    let mut buttons: HashMap<_, _> = buttons
        .into_iter()
        .enumerate()
        .map(|(i, pin)| (i, PinDriver::input(pin).unwrap()))
        .collect();

    for button in buttons.values_mut() {
        button.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    }

    loop {
        let notification = Notification::new();
        
        for (i, button) in buttons.iter_mut() {
            let waker = notification.notifier();
            let bit = 1 << i;

            unsafe {
                button.subscribe(move || { 
                    waker.notify(NonZero::new(bit).unwrap());
                });
            }

            button.enable_interrupt().unwrap();
        }

        // block until notified
        loop {
            if let Some(notification_value) = notification.wait(BLOCK) {
                let notification_value: u32 = notification_value.into();

                for (i, button) in &buttons {
                    if notification_value & (1 << i) != 0 {
                        println!("button{}: {}", i, bool::from(button.get_level()));
                        sender.send(*i as u32).unwrap();
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
            run(tx, buttons);
        });

        s.spawn(|| {
            i2c_loop(rx);
        });
    });

    Ok(())
}
