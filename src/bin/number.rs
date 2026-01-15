#![no_std]
#![no_main]
#![feature(adt_const_params)]
#![feature(const_array)]
#![feature(const_trait_impl)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Ticker, Timer};
use mypros::{self as _, *};

use embassy_sync::channel::Channel;

static IMAGE_CHANNEL: ImgChannel = Channel::new();
static BUTTON_CHANNEL: ButtonChannel = Channel::new();
//

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    {
        let a = inner_disp!(p).to_display();
        *(LED.lock().await) = Some(a);
    }
    spawner
        .spawn(blinkline(&LED, IMAGE_CHANNEL.receiver()))
        .unwrap();
    spawner
        .spawn(button(
            ButtonPress::Left,
            BUTTON_CHANNEL.sender(),
            button!(p, ButtonPress::Left),
        ))
        .unwrap();

    spawner
        .spawn(button(
            ButtonPress::Right,
            BUTTON_CHANNEL.sender(),
            button!(p, ButtonPress::Right),
        ))
        .unwrap();
    spawner
        .spawn(time_tick(ButtonPress::Left, BUTTON_CHANNEL.sender()))
        .unwrap();
    spawner
        .spawn(button_manager(
            IMAGE_CHANNEL.sender(),
            BUTTON_CHANNEL.receiver(),
        ))
        .unwrap();
}

#[embassy_executor::task]
async fn time_tick(b: ButtonPress, snd: ButtonSender) {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        snd.send(b).await;
        ticker.next().await;
    }
}

#[embassy_executor::task]
/// Watches an embassy-sync channel that transmits button presses, and sends images in response to
/// events
async fn button_manager(snd: ImgSender, recv: ButtonReceiver) {
    const DIGITS: &'static [LedArr; 10] = &DisplayPins::DIGITS;
    const LEN: usize = DIGITS.len();
    let mut i: usize = LEN;
    loop {
        let butt_event = recv.receive().await;
        let temp = match butt_event {
            ButtonPress::Right => (i + 1).min(9),
            ButtonPress::Left => i.saturating_sub(1),
        };
        if temp == i {
            continue;
        } else {
            i = temp
        }
        snd.send(LedState::NewImg(
            DIGITS[i].map(|x| x.map(|y| y.min(1) * 10)),
        ))
        .await;
    }
}

#[embassy_executor::task]
/// Paints a picture
async fn blinkline(a: &'static LedType, rec: ImgReceiver) {
    let mut img: [[u8; 5]; 5] = [[0; 5]; 5];
    let mut now = Instant::now();
    loop {
        if let Some(display_pins) = a.lock().await.as_mut() {
            display_pins.display(&img).await;
        }
        if let Ok(LedState::NewImg(new_img)) = rec.try_receive() {
            img = new_img;
            now = Instant::now();
        }
        if now.elapsed().as_millis() >= 100 {
            img = img.map(|x| x.map(|y| y.saturating_sub(1)));
            now = Instant::now();
        }
    }
}
