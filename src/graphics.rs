pub mod color;
pub mod config;

use crate::command::{Command, DataEntryMode, DeepSleepMode, IncrementAxis};
use crate::error::Error;
use crate::{DisplayInterface, Interface};
use embedded_graphics_core::Pixel;
use embedded_graphics_core::prelude::{DrawTarget, OriginDimensions, Point, Size};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use crate::graphics::color::EpdColor;
use crate::graphics::config::{Config, Rotation};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;

pub struct Graphics<SPI, BUSY, CS, DC, RESET, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    CS: OutputPin,
    DC: OutputPin,
    RESET: OutputPin,
    DELAY: DelayNs,
{
    interface: Interface<SPI, BUSY, CS, DC, RESET>,
    config: Config,
    delay: DELAY,
    black_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
    #[cfg(feature = "use_red")]
    red_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
}

impl<SPI, BUSY, CS, DC, RESET, DELAY> Graphics<SPI, BUSY, CS, DC, RESET, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    CS: OutputPin,
    DC: OutputPin,
    RESET: OutputPin,
    DELAY: DelayNs,
{
    pub fn new(interface: Interface<SPI, BUSY, CS, DC, RESET>, config: Config, delay: DELAY) -> Self {
        Self {
            interface,
            config,
            delay,
            black_buffer: [1; (WIDTH * HEIGHT / 8) as usize],
            #[cfg(feature = "use_red")]
            red_buffer: [0; (WIDTH * HEIGHT / 8) as usize],
        }
    }

    /// 官方数据手册记录的初始化方式
    #[allow(clippy::type_complexity)]
    fn init(&mut self) -> Result<(), Error<SPI::Error, CS::Error, DC::Error, RESET::Error>> {
        self.interface.reset(&mut self.delay)?;
        Command::SoftReset.execute(&mut self.interface)?;
        self.interface.busy_wait();

        // Send Initialization Code
        Command::DriverOutputControl(self.config.height, 0x00).execute(&mut self.interface)?;
        Command::DataEntryMode(
            DataEntryMode::IncrementXDecrementY,
            IncrementAxis::Horizontal,
        )
        .execute(&mut self.interface)?;
        Command::StartEndXPosition(0x00, (self.config.width / 8 - 1) as u8)
            .execute(&mut self.interface)?;
        Command::StartEndYPosition(0x00, self.config.height).execute(&mut self.interface)?;
        Command::BorderWaveform(0x05).execute(&mut self.interface)?;

        // Load Waveform LUT
        Command::ReadTemperatureSensor(0x80).execute(&mut self.interface)?;
        Command::DisplayUpdateControl2(0x91).execute(&mut self.interface)?;
        Command::MasterActivation.execute(&mut self.interface)?;
        self.interface.busy_wait();
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn update(&mut self) -> Result<(), Error<SPI::Error, CS::Error, DC::Error, RESET::Error>> {
        Command::XAddress(0).execute(&mut self.interface)?;
        Command::YAddress(0).execute(&mut self.interface)?;
        Command::WriteRamBW.execute(&mut self.interface)?;
        self.interface.send_data(&self.black_buffer)?;
        #[cfg(feature = "use_red")]
        {
            Command::WriteRamRed.execute(&mut self.interface)?;
            self.interface.send_data(&self.red_buffer)?;
        }
        Command::DisplayUpdateControl2(0xC7).execute(&mut self.interface)?;
        Command::MasterActivation.execute(&mut self.interface)?;
        self.interface.busy_wait();
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn deep_sleep(
        &mut self,
        deep_sleep_mode: DeepSleepMode,
    ) -> Result<(), Error<SPI::Error, CS::Error, DC::Error, RESET::Error>> {
        Command::DeepSleepMode(deep_sleep_mode).execute(&mut self.interface)?;
        Ok(())
    }

    fn set_pixel(&mut self, x: u32, y: u32, color: EpdColor) {
        let (index, bit) = rotation(
            x,
            y,
            self.config.width as u32,
            self.config.height as u32,
            self.config.rotation,
        );
        let index = index as usize;

        match color {
            EpdColor::Black => {
                self.black_buffer[index] &= !bit;
                #[cfg(feature = "use_red")]
                {
                    self.red_buffer[index] &= !bit;
                }
            }
            EpdColor::White => {
                self.black_buffer[index] |= bit;
                #[cfg(feature = "use_red")]
                {
                    self.red_buffer[index] &= !bit;
                }
            }
            #[cfg(feature = "use_red")]
            EpdColor::Red => {
                self.black_buffer[index] |= bit;
                self.red_buffer[index] |= bit;
            }
        }
    }
}

impl<SPI, BUSY, CS, DC, RESET, DELAY> OriginDimensions for Graphics<SPI, BUSY, CS, DC, RESET, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    CS: OutputPin,
    DC: OutputPin,
    RESET: OutputPin,
    DELAY: DelayNs,
{
    fn size(&self) -> Size {
        let width = self.config.width as u32;
        let height = self.config.height as u32;
        match self.config.rotation {
            Rotation::Rotate0 | Rotation::Rotate180 => {
                Size::new(width, height)
            }
            Rotation::Rotate90 | Rotation::Rotate270 => {
                Size::new(height, width)
            }
        }
    }
}

impl<SPI, BUSY, CS, DC, RESET, DELAY> DrawTarget for Graphics<SPI, BUSY, CS, DC, RESET, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    CS: OutputPin,
    DC: OutputPin,
    RESET: OutputPin,
    DELAY: DelayNs,
{
    type Color = EpdColor;
    type Error = Error<SPI::Error, CS::Error, DC::Error, RESET::Error>;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let Pixel(Point {x, y}, color) = pixel;
            self.set_pixel(x as u32, y as u32, color);
        }
        Ok(())
    }
}

fn rotation(x: u32, y: u32, width: u32, height: u32, rotation: Rotation) -> (u32, u8) {
    match rotation {
        Rotation::Rotate0 => (x / 8 + (width / 8) * y, 0x80 >> (x % 8)),
        Rotation::Rotate90 => ((width - 1 - y) / 8 + (width / 8) * x, 0x01 << (y % 8)),
        Rotation::Rotate180 => (
            ((width / 8) * height - 1) - (x / 8 + (width / 8) * y),
            0x01 << (x % 8),
        ),
        Rotation::Rotate270 => (y / 8 + (height - 1 - x) * (width / 8), 0x80 >> (y % 8)),
    }
}