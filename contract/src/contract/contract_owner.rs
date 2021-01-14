use crate::interface::{AccountManagement, ContractFinancials, ContractOwner, YoctoNear};
//required in order for near_bindgen macro to work outside of lib.rs
use crate::errors::contract_owner::{
    INSUFFICIENT_FUNDS_FOR_OWNER_STAKING, INSUFFICIENT_FUNDS_FOR_OWNER_WITHDRAWAL,
    TRANSFER_TO_NON_REGISTERED_ACCOUNT,
};
use crate::interface::contract_owner::events::OwnershipTransferred;
use crate::near::log;
use crate::*;
use near_sdk::{json_types::ValidAccountId, near_bindgen, Promise};

#[near_bindgen]
impl ContractOwner for StakeTokenContract {
    fn owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    fn transfer_ownership(&mut self, new_owner: ValidAccountId) {
        self.assert_predecessor_is_owner();
        assert!(
            self.account_registered(new_owner.clone()),
            TRANSFER_TO_NON_REGISTERED_ACCOUNT,
        );

        let previous_owner = self.owner_id.clone();
        self.owner_id = new_owner.into();

        log(OwnershipTransferred {
            from: &previous_owner,
            to: &self.owner_id,
        });
    }

    fn stake_all_owner_balance(&mut self) -> YoctoNear {
        self.assert_predecessor_is_owner();
        let mut account = self.registered_account(&self.owner_id);
        let balances = self.balances();
        let owner_available_balance = balances.contract_owner_available_balance;
        assert!(owner_available_balance.value() > 0, "owner balance is zero");
        self.deposit_near_for_account_to_stake(
            &mut account,
            owner_available_balance.value().into(),
        );
        self.save_registered_account(&account);
        owner_available_balance
    }

    fn stake_owner_balance(&mut self, amount: YoctoNear) {
        self.assert_predecessor_is_owner();
        let mut account = self.registered_account(&self.owner_id);
        let owner_available_balance = self.balances().contract_owner_available_balance;
        assert!(
            owner_available_balance.value() >= amount.value(),
            INSUFFICIENT_FUNDS_FOR_OWNER_STAKING
        );
        self.deposit_near_for_account_to_stake(&mut account, amount.into());
        self.save_registered_account(&account);
    }

    fn withdraw_all_owner_balance(&mut self) -> YoctoNear {
        self.assert_predecessor_is_owner();
        let owner_available_balance = self.balances().contract_owner_available_balance;
        Promise::new(self.owner_id.clone()).transfer(owner_available_balance.value());
        owner_available_balance
    }

    fn withdraw_owner_balance(&mut self, amount: YoctoNear) {
        self.assert_predecessor_is_owner();
        let owner_available_balance = self.balances().contract_owner_available_balance;
        assert!(
            owner_available_balance.value() >= amount.value(),
            INSUFFICIENT_FUNDS_FOR_OWNER_WITHDRAWAL
        );
        Promise::new(self.owner_id.clone()).transfer(amount.value());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interface::ContractFinancials;
    use crate::near::YOCTO;
    use crate::test_utils::*;
    use near_sdk::test_utils::get_created_receipts;
    use near_sdk::{testing_env, MockedBlockchain};
    use std::convert::TryFrom;

    #[test]
    fn transfer_ownership_success() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);

        context.attached_deposit = YOCTO;
        testing_env!(context.clone());
        contract.register_account();

        context.predecessor_account_id = contract.owner_id.clone();
        testing_env!(context.clone());
        contract.transfer_ownership(ValidAccountId::try_from(account_id).unwrap());
        assert_eq!(&contract.owner_id, account_id)
    }

    #[test]
    #[should_panic(expected = "contract ownership can only be transferred to a registered account")]
    fn transfer_ownership_to_non_registered_account() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);

        context.predecessor_account_id = contract.owner_id.clone();
        testing_env!(context.clone());
        contract.transfer_ownership(ValidAccountId::try_from(account_id).unwrap());
    }

    #[test]
    #[should_panic(expected = "contract call is only allowed by the contract owner")]
    fn transfer_ownership_from_non_owner() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);

        testing_env!(context.clone());
        contract.transfer_ownership(ValidAccountId::try_from(account_id).unwrap());
    }

    #[test]
    fn withdraw_all_owner_balance_success() {
        let mut test_context = TestContext::new(None);
        let mut context = test_context.context.clone();
        let contract = &mut test_context.contract;

        let owner_available_balance = contract.balances().contract_owner_available_balance;

        context.predecessor_account_id = contract.owner_id();
        testing_env!(context.clone());
        contract.withdraw_all_owner_balance();
        let receipts = deserialize_receipts(&get_created_receipts());
        assert_eq!(receipts.len(), 1);
        let receipt = receipts.first().unwrap();
        println!("{:#?}", receipt);
        assert_eq!(receipt.receiver_id, contract.owner_id());
        if let Action::Transfer { deposit } = receipt.actions.first().unwrap() {
            assert_eq!(owner_available_balance.value(), *deposit);
        } else {
            panic!("expected transfer action");
        }
    }

    #[test]
    fn withdraw_owner_balance_success() {
        let mut test_context = TestContext::new(None);
        let mut context = test_context.context.clone();
        let contract = &mut test_context.contract;

        context.predecessor_account_id = contract.owner_id();
        testing_env!(context.clone());
        contract.withdraw_owner_balance(YOCTO.into());
        let receipts = deserialize_receipts(&get_created_receipts());
        assert_eq!(receipts.len(), 1);
        let receipt = receipts.first().unwrap();
        println!("{:#?}", receipt);
        assert_eq!(receipt.receiver_id, contract.owner_id());
        if let Action::Transfer { deposit } = receipt.actions.first().unwrap() {
            assert_eq!(YOCTO, *deposit);
        } else {
            panic!("expected transfer action");
        }
    }

    #[test]
    #[should_panic(expected = "contract call is only allowed by the contract owner")]
    fn withdraw_all_owner_balance_called_by_non_owner() {
        let mut test_context = TestContext::new(None);
        test_context.contract.withdraw_all_owner_balance();
    }

    #[test]
    #[should_panic(expected = "contract call is only allowed by the contract owner")]
    fn withdraw_owner_balance_called_by_non_owner() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);
        contract.withdraw_owner_balance(YOCTO.into());
    }

    #[test]
    #[should_panic(expected = "contract call is only allowed by the contract owner")]
    fn stake_owner_balance_called_by_non_owner() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);
        contract.stake_owner_balance(YOCTO.into());
    }

    #[test]
    #[should_panic(expected = "contract call is only allowed by the contract owner")]
    fn stake_all_owner_balance_called_by_non_owner() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);
        contract.stake_all_owner_balance();
    }

    #[test]
    fn stake_all_owner_balance_success() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);

        context.attached_deposit = YOCTO;
        context.predecessor_account_id = contract.owner_id();
        testing_env!(context.clone());
        contract.register_account();
        contract.stake_all_owner_balance();
        let account = contract
            .lookup_account(ValidAccountId::try_from(contract.owner_id.as_str()).unwrap())
            .unwrap();
        assert!(account.stake_batch.is_some());
    }

    #[test]
    fn stake_owner_balance_success() {
        let account_id = "alfio-zappala.near";
        let mut context = new_context(account_id);
        context.account_balance = 100 * YOCTO;
        context.is_view = false;
        testing_env!(context.clone());

        let contract_settings = default_contract_settings();
        let mut contract = StakeTokenContract::new(None, contract_settings);

        context.attached_deposit = YOCTO;
        context.predecessor_account_id = contract.owner_id();
        testing_env!(context.clone());
        contract.register_account();
        contract.stake_owner_balance(YOCTO.into());
        let account = contract
            .lookup_account(ValidAccountId::try_from(contract.owner_id.as_str()).unwrap())
            .unwrap();
        assert!(account.stake_batch.is_some());
    }
}
