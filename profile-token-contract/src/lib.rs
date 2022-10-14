use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, ext_contract, log, near_bindgen, require, AccountId, Balance,
    BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use rust_decimal::prelude::*;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    lzr_locked: u128,
    metadata: LazyOption<FungibleTokenMetadata>,
}

const RESERVE_RATIO: f64 = 0.3333333333333333;
const SLOPE: f64 = 0.003;
const TOKEN_DECIMAL: u32 = 24;
const BASE: u128 = 10;

fn get_lzr_token_contract() -> AccountId {
    "lzr.testnet".parse().unwrap()
}

#[ext_contract(ext_ft_transfer)]
pub trait LoozrFt {
    fn ft_transfer(receiver_id: AccountId, amount: U128);
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    FungibleToken,
    Metadata,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new_default_meta(owner_id: AccountId, image_icon_data: String) -> Self {
        Self::new(
            owner_id,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Loozr-CT".to_string(),
                symbol: "LZR-CT".to_string(),
                icon: Some(image_icon_data.to_string()),
                reference: None,
                reference_hash: None,
                decimals: TOKEN_DECIMAL as u8,
            },
        )
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: FungibleTokenMetadata) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            lzr_locked: 0,
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this
    }

    pub fn reserve_balance(self) -> U128 {
        self.lzr_locked.into()
    }

    // should only be called after tokens have been transfered to contract
    #[private]
    #[payable]
    pub fn ft_mint(
        &mut self,
        amount: U128,
        account_id: AccountId,
        founder_id: AccountId,
        founder_percent: U128,
    ) -> Promise {
        assert_one_yocto();

        let amount: Balance = amount.into();
        let founder_reward_percent: Balance = founder_percent.into();
        let founder_reward_amount = (amount * founder_reward_percent) / 100;
        let deposit_amount = amount - (amount * 10) / 100;

        require!(deposit_amount > 0, "Must send loozr to buy tokens");
        let tokens_minted = self.continous_mint(deposit_amount, account_id);

        ext_ft_transfer::ext(get_lzr_token_contract())
            .with_attached_deposit(1)
            .ft_transfer(founder_id, founder_reward_amount.into())
            .then(Self::ext(env::current_account_id()).on_transfer_callback(tokens_minted))
    }

    #[private]
    #[payable]
    pub fn ft_burn(&mut self, sell_amount: U128, account_id: AccountId) -> Promise {
        let user_account_id: AccountId = account_id.clone();
        let cl_user_account_id: AccountId = user_account_id.clone();
        assert_one_yocto();
        let sell_amount: Balance = sell_amount.into();
        require!(sell_amount > 0, "Amount must be non-zero.");

        let balance = self.internal_unwrap_balance_of(user_account_id);
        require!(
            balance >= sell_amount,
            "Amount exceeds creator coin locked in"
        );
        require!(
            self.lzr_locked > 0
                && self.token.total_supply > 0
                && sell_amount <= self.token.total_supply,
            "Amount exceeds creator coin in supply"
        );

        let amount_in_near = Decimal::from_i128_with_scale(sell_amount as i128, TOKEN_DECIMAL);
        let lzr_locked_in_near =
            Decimal::from_i128_with_scale(self.lzr_locked as i128, TOKEN_DECIMAL);
        let current_supply_in_near =
            Decimal::from_i128_with_scale(self.token.total_supply as i128, TOKEN_DECIMAL);

        let reimburse_amount = self.continous_sale(
            current_supply_in_near,
            lzr_locked_in_near,
            amount_in_near,
            sell_amount,
            account_id,
        );
        ext_ft_transfer::ext(get_lzr_token_contract())
            .with_attached_deposit(1)
            .ft_transfer(cl_user_account_id, reimburse_amount.into())
            .then(
                Self::ext(env::current_account_id()).on_burn_transfer_callback(
                    sell_amount.into(),
                    reimburse_amount.into(),
                    env::predecessor_account_id().into(),
                    env::attached_deposit().into(),
                ),
            )
    }

    #[private]
    pub fn on_transfer_callback(&mut self,
        #[callback_result] call_result: Result<(), near_sdk::PromiseError>, tokens_minted: U128) -> PromiseOrValue<U128> {

          if call_result.is_err() {
             env::panic_str("Reserve balance overflow")
          }else {
            PromiseOrValue::Value(tokens_minted)
          }
    }

    #[private]
    pub fn on_burn_transfer_callback(
        &mut self,
        #[callback_result] call_result: Result<(), near_sdk::PromiseError>,
        sell_amount: U128,
        reimburse_amount: U128,
        account_id: AccountId,
        attached_deposit: U128,
    ) -> PromiseOrValue<U128> {
        // Return whether or not the promise succeeded using the method outlined in external.rs
        if call_result.is_err() {
            let predecessor_account_id = account_id.clone();
            self.internal_mint(sell_amount.into(), account_id);
            self.lzr_locked = self
                .lzr_locked
                .checked_add(reimburse_amount.into())
                .unwrap_or_else(|| env::panic_str("Reserve balance overflow"));

            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
            PromiseOrValue::Value(0.into())
        }else {
          PromiseOrValue::Value(reimburse_amount)
        }
    }

    fn continous_sale(
        &mut self,
        current_supply_in_near: Decimal,
        lzr_locked_in_near: Decimal,
        amount_in_near: Decimal,
        sell_amount: u128,
        account_id: AccountId,
    ) -> u128 {
        let reimburse_amount = self.calc_sales_return(
            current_supply_in_near,
            lzr_locked_in_near,
            RESERVE_RATIO,
            amount_in_near,
        );

        self.lzr_locked = self
            .lzr_locked
            .checked_sub(reimburse_amount)
            .unwrap_or_else(|| env::panic_str("Reserve balance overflow"));
        self.internal_burn(sell_amount, account_id);

        reimburse_amount
    }

    fn continous_mint(&mut self, _deposit: u128, account_id: AccountId) -> U128 {
        let amount = self.calc_purchase_return(_deposit);

        self.lzr_locked = self
            .lzr_locked
            .checked_add(_deposit)
            .unwrap_or_else(|| env::panic_str("Reserve balance overflow"));
        self.internal_mint(amount, account_id);
        amount.into()
    }

    fn calc_sales_return(
        &mut self,
        current_supply: Decimal,
        reserve_balance: Decimal,
        reserve_ratio: f64,
        _sell_amount: Decimal,
    ) -> u128 {
        //This is the formula:
        // rb * (1 - (1 - p / x)^(1/r))
        //
        // Constants
        // p = _sell_amount
        // rb = reserve_balance
        // x = current_supply
        // r = reserve_ratio

        let mut result = Decimal::from_f64(1.).unwrap() / Decimal::from_f64(reserve_ratio).unwrap();

        result = reserve_balance
            * Decimal::from_f64(
                1. - (1. - self.decimal_to_float(_sell_amount / current_supply))
                    .powf(self.decimal_to_float(result)),
            )
            .unwrap();

        let result_in_str =
            (self.decimal_to_float(result) * BASE.pow(TOKEN_DECIMAL) as f64).to_string();

        return result_in_str.parse::<u128>().unwrap();
    }

    fn calc_purchase_return(&mut self, _deposit: u128) -> u128 {
        let deposit_in_near = Decimal::from_i128_with_scale(_deposit as i128, TOKEN_DECIMAL);
        let total_supply_in_near =
            Decimal::from_i128_with_scale(self.token.total_supply as i128, TOKEN_DECIMAL);

        if self.lzr_locked == 0 {
            return self.calc_mint_polynomial(
                deposit_in_near,
                total_supply_in_near,
                RESERVE_RATIO,
                Decimal::from_f64(SLOPE).unwrap(),
            );
        }

        return self.calc_mint_bancor(deposit_in_near, total_supply_in_near);
    }

    fn calc_mint_polynomial(
        &self,
        amount: Decimal,
        current_supply: Decimal,
        reserve_ratio: f64,
        slope: Decimal,
    ) -> u128 {
        let increase_rate: u32 = 3; // 2(increase rate +1)

        //This is the formula:
        // (((((3*p)/m) + (x^3)) ^ r) - x)
        //
        // Constants
        // "3" here is n + 1, n is the rate of increase
        // p = deposit_amount
        // m = slope
        // x = current_supply
        // r = reserve_ratio

        let result = ((self.decimal_to_float(
            Decimal::from_f64((increase_rate as f64) * self.decimal_to_float(amount)).unwrap()
                / slope,
        ) + (self
            .decimal_to_float(current_supply)
            .powf(increase_rate as f64) as f64))
            .powf(reserve_ratio))
            - self.decimal_to_float(current_supply);

        return (result * BASE.pow(TOKEN_DECIMAL) as f64) as u128;
    }

    fn calc_mint_bancor(&self, amount: Decimal, current_supply: Decimal) -> u128 {
        //This is the formula:
        // x * ((1 + p / rb) ^ (r) - 1)
        //
        // Values
        // p = loozr_amount
        // rb = lzr_locked
        // x = current_supply
        // r = RESERVE_RATIO

        let lzr_locked_in_near =
            Decimal::from_i128_with_scale(self.lzr_locked as i128, TOKEN_DECIMAL);

        let mut result = amount / lzr_locked_in_near;
        result = current_supply
            * Decimal::from_f64((1. + self.decimal_to_float(result)).powf(RESERVE_RATIO) - 1.)
                .unwrap();

        return (self.decimal_to_float(result) * BASE.pow(TOKEN_DECIMAL) as f64) as u128;
    }

    fn internal_mint(&mut self, amount: Balance, account_id: AccountId) {
        let user_account_id = account_id.clone();
        let balance = self.internal_unwrap_balance_of(account_id);
        self.internal_update_account(&user_account_id, balance + amount);
        self.token.total_supply = self
            .token
            .total_supply
            .checked_add(amount)
            .unwrap_or_else(|| env::panic_str("Total supply overflow"));
    }

    fn internal_burn(&mut self, amount: Balance, account_id: AccountId) {
        let user_account_id = account_id.clone();
        let balance = self.internal_unwrap_balance_of(account_id);
        if amount > balance {
            env::panic_str("NOT ENOUGH BALANCE");
        }
        self.internal_update_account(&user_account_id, balance - amount);
        if amount > self.token.total_supply {
            env::panic_str("AMOUNT BIGGER THAN SUPPLY");
        }
        self.token.total_supply = self
            .token
            .total_supply
            .checked_sub(amount)
            .unwrap_or_else(|| env::panic_str("Total supply overflow"));
    }

    fn decimal_to_float(&self, amount: Decimal) -> f64 {
        let before = amount;
        let after = before.to_f64();

        match after {
            Some(result) => result,
            None => 0.,
        }
    }

    /// Inner method to save the given account for a given account ID.
    /// If the account balance is 0, the account is deleted instead to release storage.
    fn internal_update_account(&mut self, account_id: &AccountId, balance: u128) {
        if balance == 0 {
            self.token.accounts.remove(account_id);
        } else {
            self.token.accounts.insert(account_id, &balance);
        }
    }

    fn internal_unwrap_balance_of(&self, account_id: AccountId) -> Balance {
        match self.token.accounts.get(&account_id) {
            Some(balance) => balance,
            None => 0,
        }
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(1).into(), "".to_string());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(1))
            .build());

        if contract.ft_total_supply().0 != 0 {
            env::panic_str("INCORRECT SUPPLY");
        }
        contract.ft_mint(
            10000000000000000000000000.into(),
            accounts(1),
            accounts(2),
            10.into(),
        );
        let balance = contract.ft_total_supply();
        let creator_token_minted: u128 = 20800838230519037072244736;

        if balance.0 < 1 {
            env::panic_str("ERROR IN CONTINOUS MINTING");
        }
        if balance.0 != creator_token_minted {
            env::panic_str("INCORRECT MINTING FUNCTION");
        }
        // contract.ft_burn(21544346900318829112459264.into());

        if balance.0 != creator_token_minted {
            env::panic_str("INCORRECT MINTING FUNCTION");
        }
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut _contract = Contract::default();
        _contract.continous_mint(10, accounts(1));
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into(), "".to_string());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());

        contract.continous_mint(500000000000000000000000000, accounts(1));

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(1))
            .build());

        let transfer_amount = 15000000000000000000000000;
        contract.ft_transfer(accounts(2), transfer_amount.into(), None);

        if contract.ft_balance_of(accounts(2)).0 != transfer_amount {
            env::panic_str("BALANCE DOES NOT MATCH TRANSFER AMOUNT");
        }
    }
}
