use std::error::Error as StdError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Transaction includes account from different group")]
    TransactionGroup,

    #[error("The requested resource was not found")]
    NotFound,

    #[error("This user is not authorized to perform the requested operation")]
    Unauthorized,

    #[error("A conflicting edit occurred")]
    ConflictingEdit,

    #[error("Backend error: {0}")]
    Backend(#[source] Box<dyn StdError + Send + Sync + 'static>),
}

impl Error {
    pub fn backend<E: StdError + Send + Sync + 'static>(error: E) -> Self {
        Error::Backend(Box::new(error))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
