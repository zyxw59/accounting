//! Defines the core backend API

use crate::{
    error::Result,
    public::{account::Account, transaction::Transaction},
};

pub mod id;
pub mod user;
pub mod version;

use id::Id;

pub trait Backend {
    /// Create a new transaction
    fn create_transaction(
        &mut self,
        transaction: Transaction,
    ) -> impl Future<Output = Result<Id<Transaction>>> + Send;

    fn get_transactions_by_account(
        &mut self,
        account: Id<Account>,
    ) -> impl Future<Output = Result<Vec<Transaction>>> + Send;
}
