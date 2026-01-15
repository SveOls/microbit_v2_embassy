#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::{
    Peri,
    gpio::AnyPin,
    timer,
    twim::{self, Twim},
    uarte::Uarte,
};
use embassy_time::{Delay, Timer};
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, MagMode, MagOutputDataRate};
use mypros::{self as _, *};
use static_cell::ConstStaticCell;

const ACCEL_ADDR: u8 = 0b0011001;
const MAGNET_ADDR: u8 = 0b0011110;

const ACCEL_ID_REG: u8 = 0x0f;
const MAGNET_ID_REG: u8 = 0x4f;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    {
        let uarte = uarte!(p);
        *(WRITER.lock().await) = Some(uarte);
    }

    static RAM_BUFFER: ConstStaticCell<[u8; 16]> = ConstStaticCell::new([0; 16]);
    let aa = Twim::new(
        p.TWISPI1,
        Irqs,
        p.P0_16,
        p.P0_08,
        twim::Config::default(),
        RAM_BUFFER.take(),
    );
    // spawner.spawn(mag_acc_test(aa)).unwrap();
    spawner.spawn(i2c_t(aa, &WRITER)).unwrap();
    loop {
        // defmt::println!("test");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn i2c_t(aa: Twim<'static>, transmitter: &'static WriteType) {
    let mut sensor = Lsm303agr::new_with_i2c(aa);
    defmt::println!("ge");
    sensor.init().await.unwrap();
    // let a: Delay = timer0.into();
    defmt::println!("ge");
    sensor
        .set_accel_mode_and_odr(
            &mut Delay,
            AccelMode::HighResolution,
            AccelOutputDataRate::Hz50,
        )
        .await
        .unwrap();
    defmt::println!("ge");
    sensor
        .set_mag_mode_and_odr(&mut Delay, MagMode::HighResolution, MagOutputDataRate::Hz50)
        .await
        .unwrap();
    defmt::println!("ge");
    let mut sensor = sensor.into_mag_continuous().await.ok().unwrap();
    loop {
        loop {
            let mut stuff = [0; 3];
            let mut byte = [0; 1];
            loop {
                if let Some(transmitter) = transmitter.lock().await.as_mut() {
                    transmitter.read(&mut byte).await.unwrap();
                    transmitter.blocking_write(&mut byte).unwrap();
                    if [b'\n', b'\r'].contains(&byte[0]) {
                        transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                        transmitter.blocking_write(&mut stuff).unwrap();
                        transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                        defmt::println!("{}", str::from_utf8(&stuff).unwrap());
                        break;
                    }
                    stuff[0] = byte[0];
                    stuff.rotate_left(1);
                    defmt::println!("{}", stuff);
                }
            }
            const mag: [u8; 3] = [b'm', b'a', b'g'];
            const acc: [u8; 3] = [b'a', b'c', b'c'];
            const tmp: [u8; 3] = [b't', b'm', b'p'];
            defmt::println!("{}", stuff);
            match stuff {
                mag => {
                    sensor.mag_status().await;
                }
                acc => defmt::println!("accel!"),
                tmp => defmt::println!("tmp!"),
                _ => defmt::println!("nah!"),
            }
        }
        // if sensor.accel_status().await.unwrap().xyz_new_data() {
        //     let (x, y, z) = sensor.acceleration().await.unwrap().xyz_mg();
        //     let t = sensor.temperature().await.unwrap();
        //     // RTT instead of normal print
        //     defmt::println!("Acceleration: x {} y {} z {}", x, y, z);
        //     defmt::println!("Temperature: {}", t.degrees_celsius());
        // }
        //
        // if sensor.mag_status().await.unwrap().xyz_new_data() {
        //     let (x, y, z) = sensor.magnetic_field().await.unwrap().xyz_nt();
        //     // RTT instead of normal print
        //     defmt::println!("Magnetic: x {} y {} z {}", x, y, z);
        // }
    }
}

#[embassy_executor::task]
async fn mag_acc_test(mut aa: Twim<'static>) {
    let mut mag = [0; 1];
    let mut acc = [0; 1];
    // must be in a normal variable so things get properly moved into RAM
    let acc_id = [ACCEL_ID_REG];
    let mag_id = [MAGNET_ID_REG];

    aa.blocking_write_read(ACCEL_ADDR, &acc_id, &mut acc)
        .unwrap();
    defmt::println!("{}", acc);
    aa.blocking_write_read(MAGNET_ADDR, &mag_id, &mut mag)
        .unwrap();
    defmt::println!("{}", mag);
}
