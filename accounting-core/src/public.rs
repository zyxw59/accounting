pub use crate::error::Result;

pub mod account;
pub mod amount;
pub mod transaction;

#[non_exhaustive]
pub struct Handle {}

impl Handle {
    /// Open a new connection to the server
    pub async fn connect(_params: ConnectionParams) -> Result<Self> {
        todo!();
    }
}

pub struct ConnectionParams {}
