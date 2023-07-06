use std::{fmt, ops};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Amount {
    /// Credits are negative, debits are positive.
    // TODO: serialize this numerically; maybe also switch to actual fixed-point arithmetic.
    #[serde(with = "rust_decimal::serde::str")]
    value: Decimal,
}

impl Amount {
    pub const ZERO: Self = Amount {
        value: Decimal::ZERO,
    };

    /// Returns whether the amount is a debit amount
    pub fn is_debit(self) -> bool {
        self.value > Decimal::ZERO
    }

    /// Returns whether the amount is a credit amount
    pub fn is_credit(self) -> bool {
        self.value < Decimal::ZERO
    }

    /// Returns whether the amount is zero
    pub const fn is_zero(self) -> bool {
        self.value.is_zero()
    }

    pub fn abs(&self) -> Decimal {
        self.value.abs()
    }
}

impl fmt::Debug for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl ops::Add for Amount {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            value: self.value + other.value,
        }
    }
}

impl ops::Sub for Amount {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            value: self.value - other.value,
        }
    }
}

impl ops::Mul<Decimal> for Amount {
    type Output = Self;
    fn mul(self, other: Decimal) -> Self {
        Self {
            value: self.value * other,
        }
    }
}

impl ops::Mul<Amount> for Decimal {
    type Output = Amount;
    fn mul(self, other: Amount) -> Amount {
        Amount {
            value: self * other.value,
        }
    }
}

impl ops::Div<Decimal> for Amount {
    type Output = Self;
    fn div(self, other: Decimal) -> Self {
        Self {
            value: self.value / other,
        }
    }
}

impl ops::Neg for Amount {
    type Output = Self;
    fn neg(self) -> Self {
        Self { value: -self.value }
    }
}
