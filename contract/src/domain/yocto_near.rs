use crate::interface;
use near_sdk::json_types::U128;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};
use primitive_types::U256;
use std::fmt::{self, Display, Formatter};
use std::ops::{Add, AddAssign, Deref, DerefMut, Sub, SubAssign};

#[derive(
    BorshSerialize, BorshDeserialize, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Default,
)]
pub struct YoctoNear(pub u128);

impl From<u128> for YoctoNear {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl YoctoNear {
    pub fn value(&self) -> u128 {
        self.0
    }
}

impl From<YoctoNear> for u128 {
    fn from(value: YoctoNear) -> Self {
        value.0
    }
}

impl Deref for YoctoNear {
    type Target = u128;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for YoctoNear {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for YoctoNear {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Sub for YoctoNear {
    type Output = YoctoNear;

    fn sub(self, rhs: Self) -> Self::Output {
        YoctoNear(self.0 - rhs.0)
    }
}

impl SubAssign for YoctoNear {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

impl Add for YoctoNear {
    type Output = YoctoNear;

    fn add(self, rhs: Self) -> Self::Output {
        YoctoNear(self.0 + rhs.0)
    }
}

impl AddAssign for YoctoNear {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl From<YoctoNearValue> for YoctoNear {
    fn from(value: YoctoNearValue) -> Self {
        YoctoNear(value.value())
    }
}

impl From<interface::YoctoNear> for YoctoNear {
    fn from(value: interface::YoctoNear) -> Self {
        YoctoNear(value.value())
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct YoctoNearValue(pub U128);

impl YoctoNearValue {
    pub fn value(&self) -> u128 {
        self.0 .0
    }
}

impl From<YoctoNear> for YoctoNearValue {
    fn from(value: YoctoNear) -> Self {
        YoctoNearValue(value.0.into())
    }
}

impl From<u128> for YoctoNearValue {
    fn from(value: u128) -> Self {
        Self(value.into())
    }
}

impl Deref for YoctoNearValue {
    type Target = u128;

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

impl DerefMut for YoctoNearValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0 .0
    }
}

impl Display for YoctoNearValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0 .0.fmt(f)
    }
}

impl From<YoctoNear> for U256 {
    fn from(value: YoctoNear) -> Self {
        U256::from(value.value())
    }
}