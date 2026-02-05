#[derive(thiserror::Error, Debug)]
pub enum Error<Spi, Reset, Dc> {
    #[error("SPI error: {0}")]
    Spi(#[source] Spi),
    #[error("Reset pin error: {0}")]
    Reset(#[source] Reset),
    #[error("DC pin error: {0}")]
    Dc(#[source] Dc),
}
