use embassy_nrf::uarte::Uarte;
use embassy_nrf::uarte::UarteRx;
use embassy_nrf::uarte::UarteTx;

use super::*;

pub static WRITER: WriteType = Mutex::new(None);
pub static WRITER_RX: WriteTypeRx = Mutex::new(None);
pub static WRITER_TX: WriteTypeTx = Mutex::new(None);
pub type WriteType = Mutex<ThreadModeRawMutex, Option<Uarte<'static>>>;
pub type WriteTypeRx = Mutex<ThreadModeRawMutex, Option<UarteRx<'static>>>;
pub type WriteTypeTx = Mutex<ThreadModeRawMutex, Option<UarteTx<'static>>>;

pub type UarteChannel = Channel<ThreadModeRawMutex, CharSend, 64>;
pub type UarteSender = Sender<'static, ThreadModeRawMutex, CharSend, 64>;
pub type UarteReceiver = Receiver<'static, ThreadModeRawMutex, CharSend, 64>;

pub enum CharSend {
    Char([u8; 4]),
}
