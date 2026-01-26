#![no_std]
extern crate embedded_hal;
extern crate alloc;
extern crate embedded_graphics_core;

pub mod command;
pub mod interface;
pub mod error;
#[cfg(feature = "graphics")]
pub mod graphics;

pub use interface::DisplayInterface;
pub use interface::Interface;
