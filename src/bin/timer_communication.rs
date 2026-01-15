#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::uarte::{Config, Uarte};
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use mypros::{self as _, *};

//
static CHAR_CHANNEL: UarteChannel = Channel::new();
//

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    {
        let (uartetx, uarterx) = uarte!(p).split();
        *(WRITER_RX.lock().await) = Some(uarterx);
        *(WRITER_TX.lock().await) = Some(uartetx);
        // *(WRITER.lock().await) = Some(uarte);
    }
    #[cfg(debug_assertions)]
    {
        assert_eq!(0, Config::default().parity as u8);
        assert!(
            Config::default().baudrate.to_bits()
                == embassy_nrf::buffered_uarte::Baudrate::BAUD115200.to_bits()
        );
    }
    spawner.spawn(printo(&WRITER_TX)).unwrap();
    spawner
        .spawn(utf8_ify(&WRITER_RX, CHAR_CHANNEL.sender()))
        .unwrap();
    spawner
        .spawn(get_utf8_msg(CHAR_CHANNEL.receiver()))
        .unwrap();
}
#[embassy_executor::task]
pub async fn get_utf8_msg(rec: UarteReceiver) {
    loop {
        let CharSend::Char(b) = rec.receive().await;
        defmt::println!("{}", str::from_utf8(&b).unwrap()); // str causes 3kB of binary size 
    }
}

#[embassy_executor::task]
pub async fn utf8_ify(readmut: &'static WriteTypeRx, snd: UarteSender) {
    loop {
        let mut buf = [0u8; 4];
        let mut redb = [0u8];
        if let Some(reader) = readmut.lock().await.as_mut() {
            for i in 0..4 {
                reader.read(&mut redb).await.unwrap();
                // defmt::println!("{}", redb[0]);
                buf[i] = redb[0];
                if (i == 0 && (buf[0] >> 7) & 1 == 0) || ((buf[0] >> 6 - i) & 1 == 0 && i != 0) {
                    // // unable to find a print! version sadly, but otherwise works!
                    buf.rotate_right(3 - i);
                    // defmt::println!("{}", str::from_utf8(&buf[(2 - i)..]).unwrap());
                    snd.send(CharSend::Char(buf)).await;
                    break;
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn printo(writer: &'static WriteTypeTx) {
    let mut buf = [0; 8];
    buf.copy_from_slice(b"Hello!\r\n");
    if let Some(writer) = writer.lock().await.as_mut() {
        writer.write(&buf).await.ok();
    }
    let now = embassy_time::Instant::now();
    loop {
        if let Some(writer) = writer.lock().await.as_mut() {
            writer
                .write(&test(now.elapsed().as_millis() as u32))
                .await
                .unwrap();
            writer.write(b"\r\n").await.unwrap();
        }
        Timer::after_millis(1000).await;
    }
}

/// Returns the last 6 digits of the number, then \r \n.
fn test(mut inp: u32) -> [u8; 8] {
    let mut ret = [48; 8]; // ascii starts at 48 = 0, 49 = 1
    for i in 0..8 {
        ret[7 - i] += (inp % 10) as u8;
        inp /= 10;
    }
    ret
}
