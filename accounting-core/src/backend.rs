//! Defines the core backend API
use time::Date;

use crate::{
    error::Result,
    public::{account::Account, transaction::Transaction},
};

pub mod id;
pub mod user;
pub mod version;

use id::{Id, WithId};

pub trait Backend {
    /// Create a new transaction
    fn create_transaction(
        &mut self,
        transaction: Transaction,
    ) -> impl Future<Output = Result<Id<Transaction>>> + Send;

    /// Get all transactions
    fn get_transactions(&self) -> impl Future<Output = Result<Vec<WithId<Transaction>>>> + Send;

    /// Get all transactions involving the specified account
    fn get_transactions_by_account(
        &self,
        account: Id<Account>,
    ) -> impl Future<Output = Result<Vec<WithId<Transaction>>>> + Send;

    /// Gets a list of all accounts
    fn get_all_accounts(&self) -> impl Future<Output = Result<Vec<WithId<Account>>>> + Send;

    /// Get the metadata and balance of an account. If `as_of` is specified, returns the balance as
    /// of the end of the specified day.
    fn get_account(
        &self,
        account: Id<Account>,
        as_of: Option<Date>,
    ) -> impl Future<Output = Result<WithId<Account>>> + Send;
}
