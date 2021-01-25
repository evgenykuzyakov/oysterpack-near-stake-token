use crate::config::{GAS_FOR_DATA_DEPENDENCY, GAS_FOR_PROMISE};
use crate::domain::Gas;
use crate::interface::ResolveTransferCall;
use crate::*;
use crate::{
    core::Hash,
    domain::{YoctoStake, TGAS},
    interface::{FungibleToken, Memo, TokenAmount, TransferCallMessage},
    near::NO_DEPOSIT,
};
use near_sdk::{
    env, ext_contract, json_types::ValidAccountId, log, near_bindgen, serde_json, Promise,
    PromiseResult,
};
#[allow(unused_imports)]
use near_sdk::{AccountId, PromiseOrValue};

#[near_bindgen]
impl FungibleToken for StakeTokenContract {
    #[payable]
    fn ft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        amount: TokenAmount,
        _memo: Option<Memo>,
    ) {
        assert_yocto_near_attached();
        assert_token_amount_not_zero(&amount);

        let stake_amount: YoctoStake = amount.value().into();

        let mut sender = self.predecessor_registered_account();
        sender.apply_stake_debit(stake_amount);
        sender.apply_near_credit(1.into());

        let mut receiver = self.registered_account(receiver_id.as_ref());
        receiver.apply_stake_credit(stake_amount);

        self.save_registered_account(&sender);
        self.save_registered_account(&receiver);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: TokenAmount,
        msg: TransferCallMessage,
        _memo: Option<Memo>,
    ) -> Promise {
        self.ft_transfer(receiver_id.clone(), amount.clone(), _memo);

        let resolve_transfer_gas: Gas = TGAS * 10;
        let gas = {
            env::prepaid_gas()
                - env::used_gas()
                - resolve_transfer_gas.value()
                - (GAS_FOR_PROMISE * 2).value()
                - GAS_FOR_DATA_DEPENDENCY.value()
        };

        ext_transfer_receiver::ft_on_transfer(
            env::predecessor_account_id(),
            amount.clone(),
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT.value(),
            gas,
        )
        .then(ext_resolve_transfer_call::ft_resolve_transfer_call(
            env::predecessor_account_id(),
            receiver_id.as_ref().to_string(),
            amount,
            &env::current_account_id(),
            NO_DEPOSIT.value(),
            resolve_transfer_gas.value(),
        ))
    }

    fn ft_total_supply(&self) -> TokenAmount {
        self.total_stake.amount().value().into()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> TokenAmount {
        self.accounts
            .get(&Hash::from(account_id))
            .map_or_else(TokenAmount::default, |account| {
                account.stake.map_or_else(TokenAmount::default, |balance| {
                    balance.amount().value().into()
                })
            })
    }
}

#[near_bindgen]
impl ResolveTransferCall for StakeTokenContract {
    #[private]
    fn ft_resolve_transfer_call(
        &mut self,
        sender_id: ValidAccountId,
        receiver_id: ValidAccountId,
        amount: TokenAmount,
    ) -> PromiseOrValue<TokenAmount> {
        assert_eq!(
            env::promise_results_count(),
            1,
            "transfer call recipient should have returned unused transfer amount"
        );
        let unused_amount: TokenAmount = match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                serde_json::from_slice(&result).expect("unsued token amount")
            }
            _ => 0.into(),
        };

        let unused_amount = if unused_amount.value() > amount.value() {
            log!(
                "WARNING: unused_amount({}) > amount({}) - refunding full amount back to sender",
                unused_amount,
                amount
            );
            amount
        } else {
            unused_amount
        };

        let refund_amount = if unused_amount.value() > 0 {
            log!("receiver returned unused amount: {}", unused_amount);
            let mut sender = self.registered_account(sender_id.as_ref());
            let mut receiver = self.registered_account(receiver_id.as_ref());
            match receiver.stake.as_mut() {
                Some(balance) => {
                    let refund_amount = if balance.amount().value() < unused_amount.value() {
                        log!("ERROR: partial refund will be applied because receiver STAKE balance is less than specified unused amount");
                        balance.amount()
                    } else {
                        unused_amount.value().into()
                    };
                    receiver.apply_stake_debit(refund_amount);
                    sender.apply_stake_credit(refund_amount);

                    self.save_registered_account(&receiver);
                    self.save_registered_account(&sender);
                    log!("sender has been refunded: {}", refund_amount.value());
                    refund_amount.value().into()
                }
                None => {
                    log!("ERROR: receiver STAKE balance is zero");
                    0.into()
                }
            }
        } else {
            unused_amount
        };
        PromiseOrValue::Value(refund_amount)
    }
}

fn assert_yocto_near_attached() {
    assert_eq!(
        env::attached_deposit(),
        1,
        "exactly 1 yoctoNEAR must be attached"
    )
}

fn assert_token_amount_not_zero(amount: &TokenAmount) {
    assert!(amount.value() > 0, "amount must not be zero")
}

#[ext_contract(ext_transfer_receiver)]
pub trait ExtTransferReceiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: TokenAmount,
        msg: TransferCallMessage,
    ) -> PromiseOrValue<TokenAmount>;
}

#[ext_contract(ext_resolve_transfer_call)]
pub trait ExtResolveTransferCall {
    fn ft_resolve_transfer_call(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: TokenAmount,
    ) -> PromiseOrValue<TokenAmount>;
}

/// and if anyone is interested I implemented NEP-141 in my STAKE project:
//
// [FungibleToken interface] (https://github.com/oysterpack/oysterpack-near-stake-token/blob/main/contract/src/interface/fungible_token.rs)
// [FungibleToken implementation](https://github.com/oysterpack/oysterpack-near-stake-token/blob/main/contract/src/contract/fungible_token.rs)
//
// NOTE: this is a sneak preview - the code is not tested yet
#[cfg(test)]
mod test_transfer {

    use super::*;
    use crate::interface::AccountManagement;
    use crate::near::YOCTO;
    use crate::test_utils::*;
    use near_sdk::{testing_env, MockedBlockchain};

    #[test]
    pub fn transfer_ok() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        assert!(contract.account_registered(to_valid_account_id(sender_id)));
        assert!(contract.account_registered(to_valid_account_id(receiver_id)));

        assert_eq!(contract.ft_total_supply(), 0.into());
        assert_eq!(
            contract.ft_balance_of(to_valid_account_id(sender_id)),
            0.into()
        );
        assert_eq!(
            contract.ft_balance_of(to_valid_account_id(receiver_id)),
            0.into()
        );

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1; // 1 yoctoNEAR is required to transfer
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );

        // Assert
        assert_eq!(contract.ft_total_supply().value(), total_supply.value());
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(sender_id))
                .value(),
            total_supply.value() - transfer_amount
        );
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(receiver_id))
                .value(),
            transfer_amount
        );
        let sender = contract.predecessor_registered_account();
        assert_eq!(sender.near.unwrap().amount().value(), 1,
                   "expected the attached 1 yoctoNEAR for the transfer to be credited to the account's NEAR balance");

        // Act - transfer with memo
        testing_env!(context.clone());
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            Some("memo".into()),
        );
        let sender = contract.predecessor_registered_account();
        assert_eq!(sender.near.unwrap().amount().value(), 2,
                   "expected the attached 1 yoctoNEAR for the transfer to be credited to the account's NEAR balance");

        // Assert
        assert_eq!(contract.ft_total_supply().value(), total_supply.value());
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(sender_id))
                .value(),
            total_supply.value() - (transfer_amount * 2)
        );
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(receiver_id))
                .value(),
            transfer_amount * 2
        );
    }

    #[test]
    #[should_panic(expected = "account is not registered: sender.near")]
    fn sender_not_registered() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = "sender.near"; // not registered
        let receiver_id = test_ctx.account_id; // registered

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1; // 1 yoctoNEAR is required to transfer
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "account is not registered: receiver.near")]
    fn receiver_not_registered() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id; // registered
        let receiver_id = "receiver.near"; // registered

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1; // 1 yoctoNEAR is required to transfer
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "exactly 1 yoctoNEAR must be attached")]
    pub fn zero_yocto_near_attached() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "exactly 1 yoctoNEAR must be attached")]
    pub fn two_yocto_near_attached() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 2;
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "amount must not be zero")]
    pub fn zero_transfer_amount() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        let transfer_amount = 0;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "account STAKE balance is too low to fulfill request")]
    pub fn sender_balance_with_insufficient_funds() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(1 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        let transfer_amount = 2 * YOCTO;
        contract.ft_transfer(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            None,
        );
    }
}

#[cfg(test)]
mod test_transfer_call {
    use super::*;
    use crate::interface::AccountManagement;
    use crate::near::YOCTO;
    use crate::test_utils::*;
    use near_sdk::{serde::Deserialize, serde_json, testing_env, MockedBlockchain};

    #[test]
    pub fn transfer_ok() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        assert!(contract.account_registered(to_valid_account_id(sender_id)));
        assert!(contract.account_registered(to_valid_account_id(receiver_id)));

        assert_eq!(contract.ft_total_supply(), 0.into());
        assert_eq!(
            contract.ft_balance_of(to_valid_account_id(sender_id)),
            0.into()
        );
        assert_eq!(
            contract.ft_balance_of(to_valid_account_id(receiver_id)),
            0.into()
        );

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1; // 1 yoctoNEAR is required to transfer
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        let msg = TransferCallMessage::from("pay");
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            msg.clone(),
            None,
        );

        // Assert
        assert_eq!(contract.ft_total_supply().value(), total_supply.value());
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(sender_id))
                .value(),
            total_supply.value() - transfer_amount
        );
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(receiver_id))
                .value(),
            transfer_amount
        );
        let sender = contract.predecessor_registered_account();
        assert_eq!(sender.near.unwrap().amount().value(), 1,
                   "expected the attached 1 yoctoNEAR for the transfer to be credited to the account's NEAR balance");

        let receipts = deserialize_receipts();
        assert_eq!(receipts.len(), 2);
        {
            let receipt = &receipts[0];
            match &receipt.actions[0] {
                Action::FunctionCall {
                    method_name,
                    args,
                    deposit,
                    gas,
                } => {
                    assert_eq!(method_name, "ft_on_transfer");
                    assert_eq!(*deposit, 0);
                    let args: TransferCallArgs = serde_json::from_str(args).unwrap();
                    assert_eq!(args.sender_id, to_valid_account_id(sender_id));
                    assert_eq!(args.amount, transfer_amount.into());
                    assert_eq!(args.msg, msg);
                    assert!(*gas >= context.prepaid_gas - (TGAS * 35).value())
                }
                _ => panic!("expected `ft_on_transfer` function call"),
            }
        }

        // Act - transfer with memo
        testing_env!(context.clone());
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            Some("memo".into()),
        );
        let sender = contract.predecessor_registered_account();
        assert_eq!(sender.near.unwrap().amount().value(), 2,
                   "expected the attached 1 yoctoNEAR for the transfer to be credited to the account's NEAR balance");

        // Assert
        assert_eq!(contract.ft_total_supply().value(), total_supply.value());
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(sender_id))
                .value(),
            total_supply.value() - (transfer_amount * 2)
        );
        assert_eq!(
            contract
                .ft_balance_of(to_valid_account_id(receiver_id))
                .value(),
            transfer_amount * 2
        );
    }

    #[test]
    #[should_panic(expected = "account is not registered: sender.near")]
    fn sender_not_registered() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = "sender.near"; // not registered
        let receiver_id = test_ctx.account_id; // registered

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1; // 1 yoctoNEAR is required to transfer
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "account is not registered: receiver.near")]
    fn receiver_not_registered() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id; // registered
        let receiver_id = "receiver.near"; // registered

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1; // 1 yoctoNEAR is required to transfer
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "exactly 1 yoctoNEAR must be attached")]
    pub fn zero_yocto_near_attached() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "exactly 1 yoctoNEAR must be attached")]
    pub fn two_yocto_near_attached() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 2;
        testing_env!(context.clone());
        let transfer_amount = 10 * YOCTO;
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "amount must not be zero")]
    pub fn zero_transfer_amount() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(100 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        let transfer_amount = 0;
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            None,
        );
    }

    #[test]
    #[should_panic(expected = "account STAKE balance is too low to fulfill request")]
    pub fn sender_balance_with_insufficient_funds() {
        // Arrange
        let mut test_ctx = TestContext::with_registered_account();
        let contract = &mut test_ctx.contract;

        let sender_id = test_ctx.account_id;
        let receiver_id = "receiver.near";

        // register receiver account
        {
            let mut context = test_ctx.context.clone();
            context.predecessor_account_id = receiver_id.to_string();
            context.attached_deposit = YOCTO;
            testing_env!(context);
            contract.register_account();
        }

        // credit the sender with STAKE
        let mut sender = contract.registered_account(sender_id);
        let total_supply = YoctoStake(1 * YOCTO);
        sender.apply_stake_credit(total_supply);
        contract.total_stake.credit(total_supply);
        contract.save_registered_account(&sender);

        // Act - transfer with no memo
        let mut context = test_ctx.context.clone();
        context.predecessor_account_id = sender_id.to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        let transfer_amount = 2 * YOCTO;
        contract.ft_transfer_call(
            to_valid_account_id(receiver_id),
            transfer_amount.into(),
            "pay".into(),
            None,
        );
    }

    #[derive(Deserialize, Debug)]
    #[serde(crate = "near_sdk::serde")]
    struct TransferCallArgs {
        sender_id: ValidAccountId,
        amount: TokenAmount,
        msg: TransferCallMessage,
    }
}
