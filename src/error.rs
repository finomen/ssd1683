#[derive(Debug, thiserror::Error)]
pub enum Error<Spi, Cs, Dc, Reset> {
    #[error("SPI error: {0}")]
    Spi(#[source] Spi),
    #[error("CS pin error: {0}")]
    Cs(#[source] Cs),
    #[error("DC pin error: {0}")]
    Dc(#[source] Dc),
    #[error("Reset pin error: {0}")]
    Reset(#[source] Reset),
}