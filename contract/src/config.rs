use crate::near::YOCTO;
use crate::{
    domain::{Gas, YoctoNear, TGAS},
    interface,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
};

/// min contract balance required above the contract's locked balance used for storage staking to
/// ensure the contract is operational
pub const CONTRACT_MIN_OPERATIONAL_BALANCE: YoctoNear = YoctoNear(YOCTO);

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct Config {
    storage_cost_per_byte: YoctoNear,
    gas_config: GasConfig,

    /// percentage of contract gas rewards that are distributed to the contract owner
    /// - the rest of the contract earnings are staked to boost the staking rewards for user accounts
    /// - must be a number between 0-100
    contract_owner_earnings_percentage: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // 1E20 yoctoNEAR (0.00001 NEAR) per byte or 10kb per NEAR token
            // https://docs.near.org/docs/concepts/storage
            storage_cost_per_byte: 100_000_000_000_000_000_000.into(),
            gas_config: GasConfig::default(),
            contract_owner_earnings_percentage: 50,
        }
    }
}

impl Config {
    pub fn storage_cost_per_byte(&self) -> YoctoNear {
        self.storage_cost_per_byte
    }

    pub fn gas_config(&self) -> GasConfig {
        self.gas_config
    }

    /// percentage of contract gas rewards that are distributed to the contract owner
    /// - the rest of the contract earnings are staked to boost the staking rewards for user accounts
    /// - must be a number between 0-100
    pub fn contract_owner_earnings_percentage(&self) -> u8 {
        self.contract_owner_earnings_percentage
    }

    /// ## Panics
    /// if validation fails
    pub fn merge(&mut self, config: interface::Config) {
        if let Some(storage_cost_per_byte) = config.storage_cost_per_byte {
            assert!(
                storage_cost_per_byte.value() > 0,
                "storage_cost_per_byte must be > 0"
            );
            self.storage_cost_per_byte = storage_cost_per_byte.value().into();
        }
        if let Some(gas_config) = config.gas_config {
            self.gas_config.merge(gas_config, true);
        }
    }

    /// performas no validation
    pub fn force_merge(&mut self, config: interface::Config) {
        if let Some(storage_cost_per_byte) = config.storage_cost_per_byte {
            self.storage_cost_per_byte = storage_cost_per_byte.value().into();
        }
        if let Some(gas_config) = config.gas_config {
            self.gas_config.merge(gas_config, false);
        }
    }
}

/// Basic compute.
pub const GAS_BASE_COMPUTE: Gas = Gas(5_000_000_000_000);
/// Fee for function call promise.
pub const GAS_FOR_PROMISE: Gas = Gas(5_000_000_000_000);
/// Fee for the `.then` call.
pub const GAS_FOR_DATA_DEPENDENCY: Gas = Gas(10_000_000_000_000);

fn assert_gas_range(gas: Gas, min: u8, max: u8, field: &str) {
    assert!(
        gas >= TGAS * min as u64 && gas <= TGAS * max as u64,
        "{} must be within {} - {} TGas",
        field,
        min,
        max
    );
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct GasConfig {
    staking_pool: StakingPoolGasConfig,
    callbacks: CallBacksGasConfig,
}

impl GasConfig {
    pub fn staking_pool(&self) -> StakingPoolGasConfig {
        self.staking_pool
    }

    pub fn callbacks(&self) -> CallBacksGasConfig {
        self.callbacks
    }

    /// if validate is true, then merge performs some sanity checks on the config to
    /// catch mis-configurations.
    ///
    /// ## Panics
    /// if validation fails
    pub fn merge(&mut self, config: interface::GasConfig, validate: bool) {
        if let Some(config) = config.callbacks {
            self.callbacks.merge(config, validate);
        }
        if let Some(config) = config.staking_pool {
            self.staking_pool.merge(config, validate);
        }

        if validate {
            // check that the numbers add up for cross-contract workflows
            assert!(
                self.callbacks.on_run_stake_batch
                    >= (self.staking_pool.deposit_and_stake
                        + self.callbacks.on_deposit_and_stake
                        + (TGAS * 5)),
                "callbacks.on_run_stake_batch must be >= \
            staking_pool.deposit_and_stake + callbacks.on_deposit_and_stake + 5 TGas"
            );
            assert!(
                self.callbacks.on_run_redeem_stake_batch
                    >= (self.staking_pool.unstake + self.callbacks.on_unstake + (TGAS * 5)),
                "callbacks.on_run_redeem_stake_batch must be >= \
            staking_pool.unstake + callbacks.on_unstake + 5 TGas"
            );
            assert!(
                self.callbacks.on_redeeming_stake_pending_withdrawal
                    >= (self.staking_pool.withdraw
                        + self.callbacks.on_redeeming_stake_post_withdrawal
                        + (TGAS * 5)),
                "callbacks.on_redeeming_stake_pending_withdrawal must be >= \
            staking_pool.withdraw + callbacks.on_redeeming_stake_post_withdrawal + 5 TGas"
            );
        }
    }
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            staking_pool: Default::default(),
            callbacks: Default::default(),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct StakingPoolGasConfig {
    deposit_and_stake: Gas,
    deposit: Gas,
    stake: Gas,
    unstake: Gas,
    withdraw: Gas,
    get_account: Gas,
}

impl Default for StakingPoolGasConfig {
    fn default() -> Self {
        Self {
            get_account: TGAS * 5,
            deposit_and_stake: TGAS * 45,
            deposit: TGAS * 45,
            stake: TGAS * 45,
            unstake: TGAS * 45,
            withdraw: TGAS * 45,
        }
    }
}

impl StakingPoolGasConfig {
    pub fn deposit_and_stake(&self) -> Gas {
        self.deposit_and_stake
    }

    pub fn deposit(&self) -> Gas {
        self.deposit
    }

    pub fn stake(&self) -> Gas {
        self.stake
    }

    pub fn unstake(&self) -> Gas {
        self.unstake
    }

    pub fn withdraw(&self) -> Gas {
        self.withdraw
    }

    pub fn get_account(&self) -> Gas {
        self.get_account
    }

    pub fn merge(&mut self, config: interface::StakingPoolGasConfig, validate: bool) {
        if let Some(gas) = config.get_account {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 5, 10, "staking_pool::get_account");
            }
            self.get_account = gas;
        }
        if let Some(gas) = config.deposit_and_stake {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 40, 75, "staking_pool::deposit_and_stake");
            }
            self.deposit_and_stake = gas;
        }
        if let Some(gas) = config.deposit {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 5, 20, "staking_pool::deposit");
            }
            self.deposit = gas;
        }
        if let Some(gas) = config.stake {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 40, 75, "staking_pool::stake");
            }
            self.stake = gas;
        }
        if let Some(gas) = config.unstake {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 40, 75, "staking_pool::unstake");
            }
            self.unstake = gas;
        }
        if let Some(gas) = config.withdraw {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 40, 75, "staking_pool::withdraw");
            }
            self.withdraw = gas;
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct CallBacksGasConfig {
    on_run_stake_batch: Gas,
    on_deposit_and_stake: Gas,

    on_unstake: Gas,
    unlock: Gas,

    // used by redeem stake workflow
    on_run_redeem_stake_batch: Gas,
    on_redeeming_stake_pending_withdrawal: Gas,
    on_redeeming_stake_post_withdrawal: Gas,
}

impl CallBacksGasConfig {
    pub fn merge(&mut self, config: interface::CallBacksGasConfig, validate: bool) {
        if let Some(gas) = config.on_run_stake_batch {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 70, 150, "callbacks::on_run_stake_batch");
            }
            self.on_run_stake_batch = gas;
        }
        if let Some(gas) = config.on_deposit_and_stake {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 5, 10, "callbacks::on_deposit_and_stake");
            }
            self.on_deposit_and_stake = gas;
        }
        if let Some(gas) = config.on_unstake {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 5, 10, "callbacks::on_unstake");
            }
            self.on_unstake = gas;
        }
        if let Some(gas) = config.unlock {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 5, 10, "callbacks::unlock");
            }
            self.unlock = gas;
        }
        if let Some(gas) = config.on_run_redeem_stake_batch {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 70, 100, "callbacks::on_run_redeem_stake_batch");
            }
            self.on_run_redeem_stake_batch = gas;
        }
        if let Some(gas) = config.on_redeeming_stake_pending_withdrawal {
            let gas = gas.into();
            if validate {
                assert_gas_range(
                    gas,
                    70,
                    100,
                    "callbacks::on_redeeming_stake_pending_withdrawal",
                );
            }
            self.on_redeeming_stake_pending_withdrawal = gas;
        }
        if let Some(gas) = config.on_redeeming_stake_post_withdrawal {
            let gas = gas.into();
            if validate {
                assert_gas_range(gas, 5, 10, "callbacks::on_redeeming_stake_post_withdrawal");
            }
            self.on_redeeming_stake_post_withdrawal = gas;
        }
    }

    pub fn on_deposit_and_stake(&self) -> Gas {
        self.on_deposit_and_stake
    }

    pub fn unlock(&self) -> Gas {
        self.unlock
    }

    pub fn on_run_stake_batch(&self) -> Gas {
        self.on_run_stake_batch
    }

    pub fn on_redeeming_stake_pending_withdrawal(&self) -> Gas {
        self.on_redeeming_stake_pending_withdrawal
    }

    pub fn on_redeeming_stake_post_withdrawal(&self) -> Gas {
        self.on_redeeming_stake_post_withdrawal
    }

    pub fn on_run_redeem_stake_batch(&self) -> Gas {
        self.on_run_redeem_stake_batch
    }

    pub fn on_unstake(&self) -> Gas {
        self.on_unstake
    }
}

impl Default for CallBacksGasConfig {
    fn default() -> Self {
        Self {
            on_run_stake_batch: TGAS * 135,
            on_deposit_and_stake: TGAS * 5,

            unlock: TGAS * 4,

            on_run_redeem_stake_batch: TGAS * 85,
            on_unstake: TGAS * 5,

            on_redeeming_stake_pending_withdrawal: TGAS * 85,
            on_redeeming_stake_post_withdrawal: TGAS * 5,
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn callbacks_gas_config_merge_success() {
        let mut config = CallBacksGasConfig::default();
        config.merge(
            interface::CallBacksGasConfig {
                on_run_stake_batch: Some((TGAS * 71).into()),
                on_deposit_and_stake: Some((TGAS * 6).into()),
                on_unstake: Some((TGAS * 7).into()),
                unlock: Some((TGAS * 8).into()),
                on_run_redeem_stake_batch: Some((TGAS * 72).into()),
                on_redeeming_stake_pending_withdrawal: Some((TGAS * 73).into()),
                on_redeeming_stake_post_withdrawal: Some((TGAS * 9).into()),
            },
            true,
        );
        assert_eq!(config.on_run_stake_batch, TGAS * 71);
        assert_eq!(config.on_deposit_and_stake, TGAS * 6);
        assert_eq!(config.on_unstake, TGAS * 7);
        assert_eq!(config.unlock, TGAS * 8);
        assert_eq!(config.on_run_redeem_stake_batch, TGAS * 72);
        assert_eq!(config.on_redeeming_stake_pending_withdrawal, TGAS * 73);
        assert_eq!(config.on_redeeming_stake_post_withdrawal, TGAS * 9);
    }

    #[test]
    fn staking_pool_gas_config_merge_success() {
        let mut config = StakingPoolGasConfig::default();
        config.merge(
            interface::StakingPoolGasConfig {
                deposit_and_stake: Some((TGAS * 71).into()),
                deposit: None,
                stake: None,
                unstake: Some((TGAS * 72).into()),
                withdraw: Some((TGAS * 73).into()),
                get_account: Some((TGAS * 7).into()),
            },
            true,
        );
        assert_eq!(config.deposit_and_stake, TGAS * 71);
        assert_eq!(config.unstake, TGAS * 72);
        assert_eq!(config.withdraw, TGAS * 73);
        assert_eq!(config.get_account, TGAS * 7);
    }
}
