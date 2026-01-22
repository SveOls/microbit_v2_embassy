#![no_std]
#![no_main]
#![feature(int_format_into)]
#![feature(core_float_math)]

use core::mem;

use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_nrf::interrupt::Priority;
use embassy_nrf::twim::Twim;
use embassy_time::{Delay, Timer};
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, MagMode, MagOutputDataRate};
use mypros::{self as _, *};
use nrf_softdevice as _;
use nrf_softdevice::ble::Connection;
use nrf_softdevice::{
    Softdevice,
    ble::{
        advertisement_builder::{
            Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList,
            ServiceUuid16,
        },
        gatt_server, peripheral,
    },
    raw,
};
use static_cell::ConstStaticCell;

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[nrf_softdevice::gatt_server]
struct Server {
    bas: BatteryService,
    foo: FooService,
}

#[nrf_softdevice::gatt_service(uuid = "180f")] // 180f -> bluetooth standard "Battery Service"
struct BatteryService {
    #[characteristic(uuid = "2a19", read, notify)] // 2a19 -> bluetooth standard "Battery Level"
    battery_level: i16,
}

#[nrf_softdevice::gatt_service(uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf38")]
struct FooService {
    #[characteristic( // NOTE!! one digit is different from gatt uuid. Byte 14/16 is 63, not 62.
        uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf39",
        read,
        write,
        notify,
        indicate
    )]
    foo: u8,
    #[characteristic( // note!! one digit is different from gatt uuid. byte 14/16 is 63, not 62.
        uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf3a",
        read,
        notify,
        indicate
    )]
    mag: i32,
}

impl Server {
    pub async fn notify_foo(&self, connection: &Connection, transmitter: &'static WriteType) {
        loop {
            if let Some(transmitter) = transmitter.lock().await.as_mut() {
                let mut byte = [0u8];
                transmitter.read(&mut byte).await.unwrap();
                transmitter.blocking_write(&mut byte).unwrap();
                match self.foo.foo_notify(connection, &byte[0]) {
                    Ok(_) => defmt::info!("Sent: {}", &byte),
                    Err(e) => defmt::error!("ohohoh: {}", e),
                };
            }
            Timer::after_millis(100).await;
        }
    }
    pub async fn notify_mag(&self, connection: &Connection, aa: &mut Twim<'static>) {
        let mut sensor = Lsm303agr::new_with_i2c(aa);
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
            if sensor
                .accel_status()
                .await
                .as_ref()
                .is_ok_and(lsm303agr::Status::xyz_new_data)
            {
                let acc = sensor.acceleration().await.unwrap().xyz_mg();
                let mag = core::f32::math::sqrt(
                    acc.0 as f32 * acc.0 as f32
                        + acc.1 as f32 * acc.1 as f32
                        + acc.2 as f32 * acc.2 as f32,
                ) as i32;
                match self.foo.mag_notify(connection, &mag) {
                    Ok(_) => defmt::info!("success! {}", mag),
                    Err(e) => defmt::info!("{}", e),
                }
            }
            Timer::after_secs(1).await;
        }
    }
}

pub const DEVICE_NAME: &str = "Halla Rust";

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    defmt::println!("hello!!!");

    let p = start();
    {
        priority_uarte();
        let uarte = uarte!(p);
        *(WRITER.lock().await) = Some(uarte);
    };
    priority_twim();
    static RAM_BUFFER: ConstStaticCell<[u8; 16]> = ConstStaticCell::new([0; 16]);
    let mut aa = twima!(p, RAM_BUFFER.take());

    static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(ServiceList::Complete, &[ServiceUuid16::BATTERY])
        .full_name(DEVICE_NAME)
        .build();

    static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .services_128(
            ServiceList::Complete,
            &[0x9e7312e0_2354_11eb_9f10_fbc30a62cf38_u128.to_le_bytes()],
        )
        .build();

    let sd = Softdevice::enable(&softdevice_config(DEVICE_NAME));
    let server = Server::new(sd).unwrap();
    spawner.spawn(softdevice_task(sd)).unwrap();

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data: &ADV_DATA,
            scan_data: &SCAN_DATA,
        };
        let conn = peripheral::advertise_connectable(sd, adv, &config)
            .await
            .unwrap();

        defmt::info!("advertising done!");
        let d = server.notify_foo(&conn, &WRITER);
        let m = server.notify_mag(&conn, &mut aa);
        let e = gatt_server::run(&conn, &server, |e| match e {
            ServerEvent::Bas(e) => match e {
                BatteryServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    defmt::info!("battery notifications: {}", notifications)
                }
            },
            ServerEvent::Foo(e) => match e {
                FooServiceEvent::FooWrite(val) => {
                    defmt::info!("wrote foo: {}", val);
                    if let Err(e) = server.foo.foo_notify(&conn, &(val + 1)) {
                        defmt::info!("send notification error: {:?}", e);
                    }
                }
                FooServiceEvent::MagCccdWrite {
                    indications,
                    notifications,
                } => {
                    defmt::info!(
                        "foo indications: {}, notifications: {}",
                        indications,
                        notifications
                    )
                }
                FooServiceEvent::FooCccdWrite {
                    indications,
                    notifications,
                } => {
                    defmt::info!(
                        "foo indications: {}, notifications: {}",
                        indications,
                        notifications
                    )
                }
            },
        });
        match select(select(d, m), e).await {
            embassy_futures::select::Either::First(_) => defmt::println!("Oh?"),
            embassy_futures::select::Either::Second(e) => {
                defmt::info!("gatt_server run exited with error: {:?}", e)
            }
        }

        // defmt::println!("test");
        Timer::after_secs(1).await;
    }
}
