use std::ops;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BooleanExpr<T> {
    Value(T),
    Not(Box<Self>),
    Any(Vec<Self>),
    All(Vec<Self>),
}

impl<T> BooleanExpr<T> {
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::All(mut a), Self::All(mut b)) => {
                a.append(&mut b);
                Self::All(a)
            }
            (Self::All(mut a), other) | (other, Self::All(mut a)) => {
                a.push(other);
                Self::All(a)
            }
            (a, b) => Self::All(vec![a, b]),
        }
    }

    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Any(mut a), Self::Any(mut b)) => {
                a.append(&mut b);
                Self::Any(a)
            }
            (Self::Any(mut a), other) | (other, Self::Any(mut a)) => {
                a.push(other);
                Self::Any(a)
            }
            (a, b) => Self::Any(vec![a, b]),
        }
    }

    pub fn try_fold<U, E>(&self, folder: &impl TryFoldBoolean<T, U, E>) -> Result<U, E> {
        match self {
            BooleanExpr::All(clauses) => {
                let clauses = clauses
                    .iter()
                    .map(|expr| expr.try_fold(folder))
                    .collect::<Result<_, E>>()?;
                folder.map_all(clauses)
            }
            BooleanExpr::Any(clauses) => {
                let clauses = clauses
                    .iter()
                    .map(|expr| expr.try_fold(folder))
                    .collect::<Result<_, E>>()?;
                folder.map_any(clauses)
            }
            BooleanExpr::Not(expr) => folder.map_not(expr.try_fold(folder)?),
            BooleanExpr::Value(value) => folder.map_value(value),
        }
    }

    pub fn try_map_values<U, E>(
        &self,
        f: &impl Fn(&T) -> Result<U, E>,
    ) -> Result<BooleanExpr<U>, E> {
        match self {
            BooleanExpr::Value(v) => f(v).map(BooleanExpr::Value),
            BooleanExpr::All(clauses) => clauses
                .iter()
                .map(|expr| expr.try_map_values(f))
                .collect::<Result<_, E>>()
                .map(BooleanExpr::All),
            BooleanExpr::Any(clauses) => clauses
                .iter()
                .map(|expr| expr.try_map_values(f))
                .collect::<Result<_, E>>()
                .map(BooleanExpr::Any),
            BooleanExpr::Not(expr) => expr.try_map_values(f).map(ops::Not::not),
        }
    }
}

impl<T> ops::Not for BooleanExpr<T> {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Self::Not(a) => *a,
            a => Self::Not(Box::new(a)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Folder<AllF, AnyF, NotF, ValueF> {
    pub all_f: AllF,
    pub any_f: AnyF,
    pub not_f: NotF,
    pub value_f: ValueF,
}

pub trait TryFoldBoolean<T, U, E> {
    fn map_all(&self, clauses: Vec<U>) -> Result<U, E>;
    fn map_any(&self, clauses: Vec<U>) -> Result<U, E>;
    fn map_not(&self, expr: U) -> Result<U, E>;
    fn map_value(&self, value: &T) -> Result<U, E>;
}

impl<T, U, E, AllF, AnyF, NotF, ValueF> TryFoldBoolean<T, U, E> for Folder<AllF, AnyF, NotF, ValueF>
where
    AllF: Fn(Vec<U>) -> Result<U, E>,
    AnyF: Fn(Vec<U>) -> Result<U, E>,
    NotF: Fn(U) -> Result<U, E>,
    ValueF: Fn(&T) -> Result<U, E>,
{
    fn map_all(&self, clauses: Vec<U>) -> Result<U, E> {
        (self.all_f)(clauses)
    }
    fn map_any(&self, clauses: Vec<U>) -> Result<U, E> {
        (self.any_f)(clauses)
    }
    fn map_not(&self, expr: U) -> Result<U, E> {
        (self.not_f)(expr)
    }
    fn map_value(&self, value: &T) -> Result<U, E> {
        (self.value_f)(value)
    }
}
