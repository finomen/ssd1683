#![no_std]
extern crate embedded_hal;
extern crate alloc;

pub mod command;
pub mod interface;
pub mod error;

pub use interface::DisplayInterface;
pub use interface::Interface;
