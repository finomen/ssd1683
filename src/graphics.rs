pub mod color;
pub mod config;
mod tools;

use crate::command::{Command, DataEntryMode, DeepSleepMode, IncrementAxis};
use crate::error::Error;
use crate::graphics::color::EpdColor;
use crate::graphics::config::{Config, Rotation};
use crate::graphics::tools::{RegionIterator, calculate_dirty_area, rotation};
use crate::{DisplayInterface, Interface};
use embedded_graphics_core::Pixel;
use embedded_graphics_core::prelude::{DrawTarget, OriginDimensions, Point, Size};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;
/// 经过多少次快速刷新后进行一次完整更新
const MAX_FAST_UPDATE_TIME: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateType {
    Update,
    UpdateFast,
    UpdatePart,
}

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
    update_type: UpdateType,
    update_count: usize,
    dirty_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
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
    pub fn new(
        interface: Interface<SPI, BUSY, CS, DC, RESET>,
        config: Config,
        delay: DELAY,
    ) -> Self {
        if !config.width.is_multiple_of(8) {
            panic!("Width must be multiple of 8");
        }
        Self {
            interface,
            config,
            delay,
            update_type: UpdateType::Update,
            update_count: 0,
            dirty_buffer: [0; (WIDTH * HEIGHT / 8) as usize],
            black_buffer: [1; (WIDTH * HEIGHT / 8) as usize],
            #[cfg(feature = "use_red")]
            red_buffer: [0; (WIDTH * HEIGHT / 8) as usize],
        }
    }

    /// 官方数据手册记录的初始化方式
    #[allow(clippy::type_complexity)]
    pub fn init(&mut self) -> Result<(), Error<SPI::Error, CS::Error, DC::Error, RESET::Error>> {
        self.interface.reset(&mut self.delay)?;
        Command::SoftReset.execute(&mut self.interface)?;
        self.interface.busy_wait();

        // Send Initialization Code
        Command::DriverOutputControl(self.config.height, 0x00).execute(&mut self.interface)?;
        Command::DataEntryMode(
            DataEntryMode::IncrementYIncrementX,
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
    pub fn update(&mut self) -> Result<(), Error<SPI::Error, CS::Error, DC::Error, RESET::Error>> {
        let dirty_rect = match calculate_dirty_area(&self.dirty_buffer, self.config.width as u32) {
            None => {
                return Ok(());
            }
            Some(dirty_rect) => dirty_rect,
        };
        // 当更新范围过大时使用全局更新
        if dirty_rect.max_byte_col - dirty_rect.min_byte_col > (self.config.width / 16) as u8
            && dirty_rect.max_y - dirty_rect.min_y > self.config.height / 2
        {
            if self.update_type == UpdateType::UpdatePart {
                self.update_type = UpdateType::UpdateFast;
                Command::StartEndXPosition(0x00, (self.config.width / 8 - 1) as u8)
                    .execute(&mut self.interface)?;
                Command::StartEndYPosition(0x00, self.config.height)
                    .execute(&mut self.interface)?;
                Command::BorderWaveform(0x05).execute(&mut self.interface)?;
                Command::XAddress(0).execute(&mut self.interface)?;
                Command::YAddress(0).execute(&mut self.interface)?;
            }
            Command::WriteRamBW.execute(&mut self.interface)?;
            self.interface.send_data(&self.black_buffer)?;
            #[cfg(feature = "use_red")]
            {
                Command::WriteRamRed.execute(&mut self.interface)?;
                self.interface.send_data(&self.red_buffer)?;
            }
        } else {
            self.update_type = UpdateType::UpdatePart;
            Command::StartEndXPosition(dirty_rect.min_byte_col, dirty_rect.max_byte_col)
                .execute(&mut self.interface)?;
            Command::StartEndYPosition(dirty_rect.min_y, dirty_rect.max_y)
                .execute(&mut self.interface)?;
            Command::BorderWaveform(0x05).execute(&mut self.interface)?;
            Command::XAddress(dirty_rect.min_byte_col).execute(&mut self.interface)?;
            Command::YAddress(dirty_rect.min_y).execute(&mut self.interface)?;
            let bw_region_iter =
                RegionIterator::new(&self.black_buffer, self.config.width as usize, &dirty_rect);
            Command::WriteRamBW.execute(&mut self.interface)?;
            for region in bw_region_iter {
                self.interface.send_data(region)?;
            }
            #[cfg(feature = "use_red")]
            {
                let red_region_iter =
                    RegionIterator::new(&self.red_buffer, self.config.width as usize, &dirty_rect);
                Command::WriteRamRed.execute(&mut self.interface)?;
                for region in red_region_iter {
                    self.interface.send_data(region)?;
                }
            }
        }
        if self.update_count >= MAX_FAST_UPDATE_TIME {
            self.update_count = 0;
            self.update_type = UpdateType::Update;
        }
        match self.update_type {
            UpdateType::Update => {
                Command::DisplayUpdateControl2(0xF7).execute(&mut self.interface)?
            }
            UpdateType::UpdateFast => {
                Command::DisplayUpdateControl2(0xC7).execute(&mut self.interface)?
            }
            UpdateType::UpdatePart => {
                Command::DisplayUpdateControl2(0xFF).execute(&mut self.interface)?
            }
        }

        Command::MasterActivation.execute(&mut self.interface)?;
        self.interface.busy_wait();
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn deep_sleep(
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
        self.dirty_buffer[index] |= bit;

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
            Rotation::Rotate0 | Rotation::Rotate180 => Size::new(width, height),
            Rotation::Rotate90 | Rotation::Rotate270 => Size::new(height, width),
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
            let Pixel(Point { x, y }, color) = pixel;
            self.set_pixel(x as u32, y as u32, color);
        }
        Ok(())
    }
}
