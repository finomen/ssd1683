#[derive(Debug, Clone, Copy)]
pub enum Rotation {
    Rotate0,
    Rotate90,
    Rotate180,
    Rotate270,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) rotation: Rotation
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 400,
            height: 300,
            rotation: Rotation::Rotate180,
        }
    }
}