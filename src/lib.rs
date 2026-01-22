#![no_std]

use embassy_nrf::{
    Peri, bind_interrupts,
    gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull},
    i2s, peripherals, twim, uarte,
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex as ThreadModeRawMutex,
    channel::{Channel, Receiver, Sender},
    mutex::Mutex,
};
use embassy_time::{Instant, Timer};
// use nrf52833_hal as _; // must be here for linking stuff (error: unfdefind symbols) HAHA NOPE
// CAUSES ERRORS
use {defmt_rtt as _, embassy_nrf as _, nrf_softdevice as _, panic_probe as _};

//
//
mod display;
pub use display::*;
//
//
mod ble;
pub use ble::*;
//
//
mod buttons;
pub use buttons::*;
//
mod myuarte;
pub use myuarte::*;
//
pub fn start() -> embassy_nrf::Peripherals {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    embassy_nrf::init(config)
}

pub fn start2() -> embassy_nrf::config::Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config
}

pub fn priority_uarte() {
    use embassy_nrf::interrupt::InterruptExt;
    embassy_nrf::interrupt::UARTE0.set_priority(embassy_nrf::interrupt::Priority::P3);
}
pub fn priority_twim() {
    use embassy_nrf::interrupt::InterruptExt;
    // const MAX_IRQ: u8 = 48;
    //
    // use embassy_nrf::interrupt::Interrupt;
    // for num in 0..=MAX_IRQ {
    //     let interrupt =
    //         unsafe { core::mem::transmute::<u8, embassy_nrf::interrupt::Interrupt>(num) };
    //     let is_enabled = InterruptExt::is_enabled(interrupt);
    //     let priority = InterruptExt::get_priority(interrupt);
    //
    //     defmt::println!(
    //         "Interrupt {}: Enabled = {}, Priority = {}",
    //         num,
    //         is_enabled,
    //         priority
    //     );
    // }
    embassy_nrf::interrupt::TWISPI1.set_priority(embassy_nrf::interrupt::Priority::P3);
}

bind_interrupts!(pub struct Irqs {
    UARTE0 => uarte::InterruptHandler<peripherals::UARTE0>;
    // I2S => i2s::InterruptHandler<peripherals::I2S>;
    TWISPI1 => twim::InterruptHandler<peripherals::TWISPI1>;
});

/// Grabs LED pins and organizes them
#[macro_export]
macro_rules! inner_disp {
    ( $pp:tt ) => {
        InnerDisplayPins {
            col: [
                $pp.P0_28.into(),
                $pp.P0_11.into(),
                $pp.P0_31.into(),
                $pp.P1_05.into(),
                $pp.P0_30.into(),
            ],
            row: [
                $pp.P0_21.into(),
                $pp.P0_22.into(),
                $pp.P0_15.into(),
                $pp.P0_24.into(),
                $pp.P0_19.into(),
            ],
        }
    };
}

/// Grabs touch sensor
#[macro_export]
macro_rules! sensor {
    ( $pp:tt ) => {
        $pp.P1_04.into()
    };
}
/// Grabs the left or right button
#[macro_export]
macro_rules! button {
    ( $pp:tt, ButtonPress::Left ) => {
        $pp.P0_14.into()
    };

    ( $pp:tt, ButtonPress::Right) => {
        $pp.P0_23.into()
    };
}

#[macro_export]
macro_rules! twima {
    ( $pp:tt, $oo:expr ) => {
        embassy_nrf::twim::Twim::new(
            $pp.TWISPI1,
            Irqs,
            $pp.P0_16,
            $pp.P0_08,
            embassy_nrf::twim::Config::default(),
            $oo,
        );
    };
}
/// Creates Uarte
#[macro_export]
macro_rules! uarte {
    ( $pp:tt ) => {
        embassy_nrf::uarte::Uarte::new(
            $pp.UARTE0,
            $pp.P1_08,
            $pp.P0_06,
            Irqs,
            embassy_nrf::uarte::Config::default(),
        );
    };
}
