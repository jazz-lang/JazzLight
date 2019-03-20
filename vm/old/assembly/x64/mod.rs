pub mod reg;

pub use self::reg::*;

#[derive(Debug, Clone, Copy)]
pub struct ForwardJump {
    at: usize,
    to: usize,
}
