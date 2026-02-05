use crate::error::Error;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

const RESET_DELAY_MS: u32 = 10;

pub struct Interface<SPI: SpiDevice, BUSY: InputPin, RESET: OutputPin, DC: OutputPin> {
    /// SPI 接口
    spi: SPI,
    /// Active low busy pin (input)
    busy: BUSY,
    /// Pin for reset the controller (output)
    reset: RESET,
    /// Data/Command Control Pin (High for data, Low for command) (output)
    dc: DC,
}

/// Trait implemented by displays to provide implementation of core functionality.
pub trait DisplayInterface {
    type Error;
    /// 发送指令
    fn send_command(&mut self, command: u8) -> Result<(), Self::Error>;

    /// 发送数据
    fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// 重置控制器
    fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Self::Error>;

    /// 等待控制器空闲
    fn busy_wait(&mut self);
}

impl<SPI, BUSY, RESET, DC> Interface<SPI, BUSY, RESET, DC>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    DC: OutputPin,
{
    pub fn new(spi: SPI, busy: BUSY, reset: RESET, dc: DC) -> Self {
        Self {
            spi,
            busy,
            reset,
            dc,
        }
    }
}

impl<SPI, BUSY, RESET, DC> DisplayInterface for Interface<SPI, BUSY, RESET, DC>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    DC: OutputPin,
{
    type Error = Error<SPI::Error, RESET::Error, DC::Error>;

    fn send_command(&mut self, command: u8) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(Error::Dc)?;
        self.spi.write(&[command]).map_err(Error::Spi)?;
        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_high().map_err(Error::Dc)?;
        self.spi.write(data).map_err(Error::Spi)?;
        Ok(())
    }

    fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Self::Error> {
        self.reset.set_low().map_err(Error::Reset)?;
        delay.delay_ms(RESET_DELAY_MS);
        self.reset.set_high().map_err(Error::Reset)?;
        delay.delay_ms(RESET_DELAY_MS);
        Ok(())
    }

    fn busy_wait(&mut self) {
        while self.busy.is_high().unwrap_or_default() {}
    }
}
