use std::{num::NonZero, sync::mpsc::{self, Receiver, Sender}, time::Duration};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp_idf_svc::hal::{
    cpu::core,
    delay::BLOCK,
    gpio::{InterruptType, PinDriver},
    prelude::Peripherals,
    task::notification::Notification,
};
use futures_util::{select, FutureExt};

fn run(sender: Sender<u32>) {
    println!("Starting control_led() on core {:?}", core());

    let peripherals = Peripherals::take().unwrap();

    let mut button0 = PinDriver::input(peripherals.pins.gpio21).unwrap();
    let mut button1 = PinDriver::input(peripherals.pins.gpio0).unwrap();
    let mut button2 = PinDriver::input(peripherals.pins.gpio14).unwrap();
    let mut button3 = PinDriver::input(peripherals.pins.gpio35).unwrap();

    button0.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    button1.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    button2.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    button3.set_interrupt_type(InterruptType::AnyEdge).unwrap();

    loop {
        // prepare communication channel
        let notification = Notification::new();
        let waker = notification.notifier();
        let waker2 = notification.notifier();
        let waker3 = notification.notifier();
        let waker4 = notification.notifier();

        // register interrupt callback, here it's a closure on stack
        unsafe {
            button0
                .subscribe_nonstatic(move || {
                    waker.notify(NonZero::new(1).unwrap());
                })
                .unwrap();

            button1
                .subscribe_nonstatic(move || {
                    waker2.notify(NonZero::new(2).unwrap());
                })
                .unwrap();

            button2
                .subscribe_nonstatic(move || {
                    waker3.notify(NonZero::new(4).unwrap());
                })
                .unwrap();

            button3
                .subscribe_nonstatic(move || {
                    waker4.notify(NonZero::new(8).unwrap());
                })
                .unwrap();
        }

        // enable interrupt, will be automatically disabled after being triggered
        button0.enable_interrupt().unwrap();
        button1.enable_interrupt().unwrap();
        button2.enable_interrupt().unwrap();
        button3.enable_interrupt().unwrap();

        // block until notified
        loop {
            if let Some(a) = notification.wait(BLOCK) {
                println!();
                let a: u32 = a.into();
                if a & 1 != 0 {
                    println!("button0: {}", bool::from(button0.get_level()));
                }

                if a & 2 != 0 {
                    println!("button1: {}", bool::from(button1.get_level()));
                }

                if a & 4 != 0 {
                    println!("button2: {}", bool::from(button2.get_level()));
                }

                if a & 8 != 0 {
                    println!("button3: {}", bool::from(button3.get_level()));
                }

                sender.send(a).unwrap();
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

static SHARED: Channel::<CriticalSectionRawMutex, u32, 3> = Channel::new();

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let (tx, rx) = mpsc::channel::<u32>();

    std::thread::scope(|s| { 
        s.spawn(|| { 
            run(tx);
        });
        s.spawn(|| { 
            i2c_loop(rx); 
        });
    });

    // let thread0 = std::thread::Builder::new()
    //     .stack_size(7000)
    //     .spawn(move || {
    //         run(sender);
    //     })?;

    // let mut recv = SHARED.receiver();
    // let thread1 = std::thread::Builder::new()
    //     .stack_size(7000)
    //     .spawn(move || {
    //         i2c_loop(recv);
    //     })?;

    // thread0.join().unwrap();
    // thread1.join().unwrap();

    Ok(())
}
