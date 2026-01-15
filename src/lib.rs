#![no_std]

use embassy_nrf::{
    Peri, bind_interrupts,
    gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull},
    i2s, peripherals, twim, uarte,
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Channel, Receiver, Sender},
    mutex::Mutex,
};
use embassy_time::{Instant, Timer};
use nrf52833_hal as _; // must be here for linking stuff (error: unfdefind symbols)
use {defmt_rtt as _, panic_probe as _};

//
//
mod display;
pub use display::*;
//
//
mod buttons;
pub use buttons::*;
//
mod myuarte;
pub use myuarte::*;
//

bind_interrupts!(pub struct Irqs {
    UARTE1 => uarte::InterruptHandler<peripherals::UARTE1>;
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
/// Creates Uarte
#[macro_export]
macro_rules! uarte {
    ( $pp:tt ) => {
        Uarte::new(
            $pp.UARTE1,
            $pp.P1_08,
            $pp.P0_06,
            Irqs,
            embassy_nrf::uarte::Config::default(),
        );
    };
}
