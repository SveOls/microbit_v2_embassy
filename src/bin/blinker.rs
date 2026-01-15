#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;
use mypros::{self as _, *};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let a = inner_disp!(p).to_display();
    {
        *(LED.lock().await) = Some(a);
    }
    for i in 0..5 {
        spawner.spawn(blinker(&LED, i)).unwrap();
        Timer::after_millis(200).await;
    }
}
#[embassy_executor::task(pool_size = 5)]
/// 5 different threads are spawned, each controlling a column each. They compete to grab a lock,
///   and once they do, they light all LEDs from top to bottom. It them sleeps for 500ms, so other
///   threads can go for it.
async fn blinker(a: &'static LedType, i: usize) {
    loop {
        for j in 0..5 {
            let mut led_unlocked = a.lock().await;
            if let Some(b) = led_unlocked.as_mut() {
                b.blink_one(i, j).await;
            }
            Timer::after_millis(50).await;
        }
        Timer::after_millis(500).await;
    }
}
