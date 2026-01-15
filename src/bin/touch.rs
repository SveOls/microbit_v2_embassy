#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::{Peri, gpio::AnyPin};
use embassy_time::Timer;
use mypros::{self as _, *};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    {
        let a = inner_disp!(p).to_display();
        *(LED.lock().await) = Some(a);
    }
    let sensor: Peri<'static, AnyPin> = sensor!(p);
    spawner.spawn(touch(&TOUCH, sensor)).unwrap();
    spawner.spawn(ledlighter(&LED, &TOUCH)).unwrap();
}

#[embassy_executor::task]
async fn ledlighter(led: &'static LedType, touch: &'static TouchType) {
    loop {
        {
            if let Some(display_pins) = led.lock().await.as_mut() {
                #[allow(unused_braces)] // they ensure the lock opens, i think?
                if { *touch.lock().await } {
                    display_pins.display(&DisplayPins::HEART).await;
                } else {
                    display_pins.display(&DisplayPins::CROSS).await;
                }
            }
        }
        Timer::after_millis(2).await;
    }
}
