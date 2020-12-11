mod redeem_stake_batch;
mod redeem_stake_batch_receipt;
mod stake_account;
mod stake_batch;
mod stake_batch_receipt;
mod stake_token_value;
mod timestamped_near_balance;
mod timestamped_stake_balance;

pub use redeem_stake_batch::RedeemStakeBatch;
pub use redeem_stake_batch_receipt::RedeemStakeBatchReceipt;
pub use stake_account::StakeAccount;
pub use stake_batch::StakeBatch;
pub use stake_batch_receipt::StakeBatchReceipt;
pub use stake_token_value::StakeTokenValue;
pub use timestamped_near_balance::TimestampedNearBalance;
pub use timestamped_stake_balance::TimestampedStakeBalance;

use crate::domain;
use near_sdk::json_types::U64;
use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct YoctoNear(pub U128);

impl From<domain::YoctoNear> for YoctoNear {
    fn from(value: domain::YoctoNear) -> Self {
        Self(value.0.into())
    }
}

impl From<u128> for YoctoNear {
    fn from(value: u128) -> Self {
        Self(value.into())
    }
}

impl YoctoNear {
    pub fn value(&self) -> u128 {
        self.0 .0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct YoctoStake(pub U128);

impl From<domain::YoctoStake> for YoctoStake {
    fn from(value: domain::YoctoStake) -> Self {
        Self(value.0.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockHeight(pub U64);

impl From<domain::BlockHeight> for BlockHeight {
    fn from(value: domain::BlockHeight) -> Self {
        Self(value.0.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockTimestamp(pub U64);

impl From<domain::BlockTimestamp> for BlockTimestamp {
    fn from(value: domain::BlockTimestamp) -> Self {
        Self(value.0.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct EpochHeight(pub U64);

impl From<domain::EpochHeight> for EpochHeight {
    fn from(value: domain::EpochHeight) -> Self {
        Self(value.0.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct BatchId(pub U128);

impl From<domain::BatchId> for BatchId {
    fn from(value: domain::BatchId) -> Self {
        Self(value.0.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockTimeHeight {
    block_height: BlockHeight,
    block_timestamp: BlockTimestamp,
    epoch_height: EpochHeight,
}