use std::{fmt, ops};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(transparent))]
#[serde(transparent)]
pub struct Amount(
    /// Credits are negative, debits are positive.
    // TODO: serialize this numerically; maybe also switch to actual fixed-point arithmetic.
    #[serde(with = "rust_decimal::serde::str")]
    Decimal,
);

impl Amount {
    pub const ZERO: Self = Amount(Decimal::ZERO);

    /// Returns whether the amount is a debit amount
    pub fn is_debit(self) -> bool {
        self.0 > Decimal::ZERO
    }

    /// Returns whether the amount is a credit amount
    pub fn is_credit(self) -> bool {
        self.0 < Decimal::ZERO
    }

    /// Returns whether the amount is zero
    pub const fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn abs(&self) -> Decimal {
        self.0.abs()
    }
}

impl fmt::Debug for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl ops::Add for Amount {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl ops::Sub for Amount {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl ops::Mul<Decimal> for Amount {
    type Output = Self;
    fn mul(self, other: Decimal) -> Self {
        Self(self.0 * other)
    }
}

impl ops::Mul<Amount> for Decimal {
    type Output = Amount;
    fn mul(self, other: Amount) -> Amount {
        Amount(self * other.0)
    }
}

impl ops::Div<Decimal> for Amount {
    type Output = Self;
    fn div(self, other: Decimal) -> Self {
        Self(self.0 / other)
    }
}

impl ops::Neg for Amount {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}
