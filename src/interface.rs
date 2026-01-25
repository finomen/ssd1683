use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

const RESET_DELAY_MS: u32 = 10;

pub struct Interface<SPI: SpiDevice, BUSY: InputPin, CS: OutputPin, DC: OutputPin, RESET: OutputPin> {
    /// SPI 接口
    spi: SPI,
    /// Active low busy pin (input)
    busy: BUSY,
    /// CS (chip select) for SPI (output)
    cs: CS,
    /// Data/Command Control Pin (High for data, Low for command) (output)
    dc: DC,
    /// Pin for reset the controller (output)
    reset: RESET,
}

/// Trait implemented by displays to provide implementation of core functionality.
pub trait DisplayInterface {
    type Error;
    /// 发送指令到控制器
    fn send_command(&mut self, command: u8) -> Result<(), Self::Error>;

    /// 发送指令数据
    fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// 重置控制器
    fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Self::Error>;

    /// 等待控制器空闲
    fn busy_wait(&mut self);
}

impl<SPI, BUSY, CS, DC, RESET> Interface<SPI, BUSY, CS, DC, RESET>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    CS: OutputPin,
    DC: OutputPin,
    RESET: OutputPin,
{
    /// 创建底层控制器交互接口
    pub fn new(spi: SPI, cs: CS, busy: BUSY, dc: DC, reset: RESET) -> Self {
        Self {
            spi,
            cs,
            busy,
            dc,
            reset,
        }
    }

    fn write(
        &mut self,
        data: &[u8],
    ) -> Result<(), crate::error::Error<SPI::Error, CS::Error, DC::Error, RESET::Error>> {
        self.cs.set_low().map_err(crate::error::Error::Cs)?;
        self.spi.write(data).map_err(crate::error::Error::Spi)?;
        self.cs.set_high().map_err(crate::error::Error::Cs)?;
        Ok(())
    }
}

impl<SPI, BUSY, CS, DC, RESET> DisplayInterface for Interface<SPI, BUSY, CS, DC, RESET>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    CS: OutputPin,
    DC: OutputPin,
    RESET: OutputPin,
{
    type Error = crate::error::Error<SPI::Error, CS::Error, DC::Error, RESET::Error>;

    fn send_command(&mut self, command: u8) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(crate::error::Error::Dc)?;
        self.write(&[command])?;
        self.dc.set_high().map_err(crate::error::Error::Dc)?;

        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_high().map_err(crate::error::Error::Dc)?;
        self.write(data)
    }

    fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Self::Error> {
        self.reset.set_low().map_err(crate::error::Error::Reset)?;
        delay.delay_ms(RESET_DELAY_MS);
        self.reset.set_high().map_err(crate::error::Error::Reset)?;
        delay.delay_ms(RESET_DELAY_MS);
        Ok(())
    }

    fn busy_wait(&mut self) {
        while match self.busy.is_high() {
            Ok(x) => x,
            _ => false,
        } {}
    }
}
