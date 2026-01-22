#![no_std]
#![no_main]
#![feature(int_format_into)]
#![feature(core_float_math)]

use embassy_executor::Spawner;
use embassy_nrf::{twim::Twim, uarte::Uarte};
use embassy_time::{Delay, Timer};
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, MagMode, MagOutputDataRate};
use mypros::{self as _, *};
use static_cell::ConstStaticCell;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = start();
    priority_uarte();
    priority_twim();

    {
        let uarte = uarte!(p);
        *(WRITER.lock().await) = Some(uarte);
    }

    static RAM_BUFFER: ConstStaticCell<[u8; 16]> = ConstStaticCell::new([0; 16]);
    let aa = twima!(p, RAM_BUFFER.take());

    // spawner.spawn(mag_acc_test(aa)).unwrap();
    spawner.spawn(i2c_t(aa, &WRITER)).unwrap();
    loop {
        // defmt::println!("test");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
pub async fn i2c_t(aa: Twim<'static>, transmitter: &'static WriteType) {
    let mut sensor = Lsm303agr::new_with_i2c(aa);
    while let Err(_) = sensor.init().await {
        defmt::println!("Error in initiating sensor");
        Timer::after_millis(100).await;
    }
    // let a: Delay = timer0.into();
    sensor
        .set_accel_mode_and_odr(
            &mut Delay,
            AccelMode::HighResolution,
            AccelOutputDataRate::Hz50,
        )
        .await
        .unwrap();
    sensor
        .set_mag_mode_and_odr(&mut Delay, MagMode::HighResolution, MagOutputDataRate::Hz50)
        .await
        .unwrap();
    let mut sensor = sensor.into_mag_continuous().await.ok().unwrap();
    loop {
        let mut stuff = [0; 3];
        let mut byte = [0; 1];
        loop {
            if let Some(transmitter) = transmitter.lock().await.as_mut() {
                transmitter.read(&mut byte).await.unwrap();
                transmitter.blocking_write(&mut byte).unwrap();
                if [b'\n', b'\r'].contains(&byte[0]) {
                    transmitter
                        .blocking_write(&mut "\r\nEvaluated as: ".as_bytes())
                        .unwrap();
                    transmitter.blocking_write(&mut stuff).unwrap();
                    transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                    // defmt::println!("{}", str::from_utf8(&stuff).unwrap());
                    break;
                }
                stuff[0] = byte[0];
                stuff.rotate_left(1);
                // defmt::println!("{}", stuff);
            }
        }
        const MAG: [u8; 3] = [b'm', b'a', b'g'];
        const ACC: [u8; 3] = [b'a', b'c', b'c'];
        const TMP: [u8; 3] = [b't', b'm', b'p'];
        // defmt::println!("{}", stuff);
        if let Some(transmitter) = transmitter.lock().await.as_mut() {
            let mut num: core::fmt::NumBuffer<i32> = core::fmt::NumBuffer::new();
            fn printer(
                transmitter: &mut Uarte<'static>,
                x: i32,
                y: i32,
                z: i32,
                mut num: &mut core::fmt::NumBuffer<i32>,
            ) {
                transmitter.blocking_write(&mut "x: ".as_bytes()).unwrap();
                transmitter
                    .blocking_write(&mut x.format_into(&mut num).as_bytes())
                    .unwrap();
                transmitter.blocking_write(&mut " y: ".as_bytes()).unwrap();
                transmitter
                    .blocking_write(&mut y.format_into(&mut num).as_bytes())
                    .unwrap();
                transmitter.blocking_write(&mut " z: ".as_bytes()).unwrap();
                transmitter
                    .blocking_write(&mut z.format_into(&mut num).as_bytes())
                    .unwrap();
                let (x, y, z) = (x as f32, y as f32, z as f32);
                transmitter
                    .blocking_write(&mut " mag: ".as_bytes())
                    .unwrap();
                transmitter
                    .blocking_write(
                        &mut (core::f32::math::sqrt(x * x + y * y + z * z) as i32)
                            .format_into(&mut num)
                            .as_bytes(),
                    )
                    .unwrap();
                transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
            }
            match stuff {
                MAG => {
                    if sensor
                        .mag_status()
                        .await
                        .as_ref()
                        .is_ok_and(lsm303agr::Status::xyz_new_data)
                    {
                        let (x, y, z) = sensor.magnetic_field().await.unwrap().xyz_nt();
                        printer(transmitter, x, y, z, &mut num)
                    } else {
                        defmt::println!("No mag :(");
                    }
                }
                ACC => {
                    if sensor
                        .accel_status()
                        .await
                        .as_ref()
                        .is_ok_and(lsm303agr::Status::xyz_new_data)
                    {
                        let (x, y, z) = sensor.acceleration().await.unwrap().xyz_mg();
                        printer(transmitter, x, y, z, &mut num)
                    } else {
                        defmt::println!("No acc :(");
                    }
                }
                TMP => {
                    if sensor
                        .temperature_status()
                        .await
                        .as_ref()
                        .is_ok_and(lsm303agr::TemperatureStatus::new_data)
                    {
                        let tempd = sensor.temperature().await.unwrap().degrees_celsius();
                        transmitter
                            .blocking_write(&mut "temp: ".as_bytes())
                            .unwrap();
                        transmitter
                            .blocking_write(&mut (tempd as i32).format_into(&mut num).as_bytes())
                            .unwrap();
                        transmitter.blocking_write(&mut ".".as_bytes()).unwrap();
                        transmitter
                            .blocking_write(
                                &mut ((100. * (tempd % 1.)) as i32)
                                    .format_into(&mut num)
                                    .as_bytes(),
                            )
                            .unwrap();
                        transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                        // defmt::println!("{}", tempd);
                    } else {
                        defmt::println!("No mag :(");
                    }
                }
                _ => {
                    transmitter
                        .blocking_write(&mut "available: mag, acc, tmp\r\n".as_bytes())
                        .unwrap();
                    // let mag = loop {
                    //     if sensor
                    //         .mag_status()
                    //         .await
                    //         .as_ref()
                    //         .is_ok_and(lsm303agr::Status::xyz_new_data)
                    //     {
                    //         break sensor.magnetic_field().await.unwrap().xyz_nt();
                    //     }
                    //     Timer::after_millis(10).await;
                    // };
                    // let acc = loop {
                    //     if sensor
                    //         .accel_status()
                    //         .await
                    //         .as_ref()
                    //         .is_ok_and(lsm303agr::Status::xyz_new_data)
                    //     {
                    //         break sensor.acceleration().await.unwrap().xyz_mg();
                    //     }
                    //     Timer::after_millis(10).await;
                    // };
                    // {
                    //     let (x, y, z) = (mag.0 as f32, mag.1 as f32, mag.2 as f32);
                    //     let mag = core::f32::math::sqrt(x * x + y * y + z * z);
                    //     transmitter.blocking_write(&mut "x: ".as_bytes()).unwrap();
                    //     transmitter
                    //         .blocking_write(&mut (mag as i32).format_into(&mut num).as_bytes())
                    //         .unwrap();
                    //     transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                    //     let (x2, y2, z2) = (acc.0 as f32, acc.1 as f32, acc.2 as f32);
                    //     let acc = core::f32::math::sqrt(x2 * x2 + y2 * y2 + z2 * z2);
                    //     transmitter.blocking_write(&mut "x: ".as_bytes()).unwrap();
                    //     transmitter
                    //         .blocking_write(&mut (acc as i32).format_into(&mut num).as_bytes())
                    //         .unwrap();
                    //     transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                    //     let cos = ((x * x2) + (y * y2) + (z * z2)) / (mag * acc);
                    //     defmt::println!("{}", cos);
                    //     let ang = core::f32::consts::FRAC_2_PI
                    //         - cos
                    //         - (cos * cos * cos) / 6.
                    //         - 3. * cos * cos * cos * cos * cos / 40.;
                    //     transmitter
                    //         .blocking_write(
                    //             &mut ((100. * ang) as i32).format_into(&mut num).as_bytes(),
                    //         )
                    //         .unwrap();
                    //     transmitter.blocking_write(&mut "\r\n".as_bytes()).unwrap();
                    // }
                }
            }
        }
        Timer::after_millis(10).await;
    }
}

const ACCEL_ADDR: u8 = 0b0011001;
const MAGNET_ADDR: u8 = 0b0011110;

const ACCEL_ID_REG: u8 = 0x0f;
const MAGNET_ID_REG: u8 = 0x4f;

#[embassy_executor::task]
async fn mag_acc_test(mut aa: Twim<'static>) {
    let mut mag = [0; 1];
    let mut acc = [0; 1];
    // must be in a normal variable so things get properly moved into RAM
    let acc_id = [ACCEL_ID_REG];
    let mag_id = [MAGNET_ID_REG];

    //
    // does not hang
    aa.blocking_write_read(ACCEL_ADDR, &acc_id, &mut acc)
        .unwrap();
    defmt::println!("{}", acc);
    // hangs HAHA NOT ANYMORE!!!!!!
    aa.write_read(ACCEL_ADDR, &acc_id, &mut acc).await.unwrap();
    //
    //
    defmt::println!("{}", acc);
    aa.blocking_write_read(MAGNET_ADDR, &mag_id, &mut mag)
        .unwrap();
    defmt::println!("{}", mag);
}
