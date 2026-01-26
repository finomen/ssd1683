#[cfg(not(feature = "use_red"))]
use embedded_graphics_core::pixelcolor::raw::RawU1;
#[cfg(feature = "use_red")]
use embedded_graphics_core::pixelcolor::raw::RawU2;
use embedded_graphics_core::prelude::PixelColor;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EpdColor {
    Black,
    White,
    #[cfg(feature = "use_red")]
    Red,
}

impl EpdColor {
    pub fn black_bit(&self) -> u8 {
        match self {
            EpdColor::Black => 0b0,
            EpdColor::White => 0b1,
            #[cfg(feature = "use_red")]
            EpdColor::Red => 0b1,
        }
    }

    #[cfg(feature = "use_red")]
    pub fn red_bit(&self) -> u8 {
        match self {
            EpdColor::Black | EpdColor::White => 0b0,
            EpdColor::Red => 0b1,
        }
    }
}

#[cfg(feature = "use_red")]
impl PixelColor for EpdColor {
    type Raw = RawU2;
}

#[cfg(not(feature = "use_red"))]
impl PixelColor for EpdColor {
    type Raw = RawU1;
}

impl From<u8> for EpdColor {
    fn from(value: u8) -> Self {
        match value {
            0b00 => EpdColor::Black,
            0b01 => EpdColor::White,
            #[cfg(feature = "use_red")]
            0b10 | 0b11 => EpdColor::Red,
            _ => EpdColor::White,
        }
    }
}
