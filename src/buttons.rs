use super::*;
#[derive(Copy, Clone, defmt::Format)]
pub enum ButtonPress {
    Left,
    Right,
}

pub type ButtonChannel = Channel<ThreadModeRawMutex, ButtonPress, 64>;
pub type ButtonSender = Sender<'static, ThreadModeRawMutex, ButtonPress, 64>;
pub type ButtonReceiver = Receiver<'static, ThreadModeRawMutex, ButtonPress, 64>;
#[embassy_executor::task(pool_size = 2)]
/// Waits for button events and sends them to the ButtonPress event. This is to be able to poll for
/// both keys at once.
pub async fn button(send: ButtonPress, snd: ButtonSender, b: Peri<'static, AnyPin>) {
    let mut b = Input::new(b, Pull::Up);
    loop {
        b.wait_for_low().await;
        snd.try_send(send).ok(); // all conceivable fail states would make the button press
        // irrelevant anyway.
        b.wait_for_high().await;
        Timer::after_millis(10).await;
    }
}

pub static TOUCH: TouchType = Mutex::new(false);
pub type TouchType = Mutex<ThreadModeRawMutex, bool>;

#[embassy_executor::task]
pub async fn touch(tch: &'static TouchType, p: Peri<'static, AnyPin>) {
    const TOUCH_FACTOR: u32 = 1000;
    const THRESHOLD: u32 = 300;
    const DELTA: u64 = 100;
    const PER_SECOND: u64 = 100;
    const N: u32 = 5;
    const {
        assert!(THRESHOLD <= TOUCH_FACTOR);
        assert!(N < 100);
    }
    let mut inp = Input::new(p, Pull::None);
    let mut x = [0; N as usize];
    loop {
        let mut i = (0, 0);
        for _ in 0..(1000_000 / (DELTA * PER_SECOND)) {
            match inp.get_level() {
                Level::Low => i.0 += 1,
                Level::High => i.1 += 1,
            }
            Timer::after_micros(DELTA).await;
        }
        x.rotate_right(1);
        x[0] = (TOUCH_FACTOR * i.0) / (i.0 + i.1);
        if {
            let mut touch = tch.lock().await;
            let val = x.iter().sum::<u32>() / N as u32;
            *touch = val >= THRESHOLD;
            val == 0
        } {
            inp.wait_for_low().await;
        }
    }
}
