use crate::error::WireError;

pub trait WireContract {
    fn validate_wire(&self) -> Result<(), WireError>;
}
