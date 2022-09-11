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

const DATA_IMAGE_SVG_LZR_ICON: &str = "data:image/png;base64,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";

const RESERVE_RATIO: f64 = 0.3333333333333333;
const SLOPE: f64 = 0.003;
const TOKEN_DECIMAL: u32 = 24;
const BASE: u128 = 10;

fn get_lzr_contract() -> AccountId {
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
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        Self::new(
            owner_id,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Loozr-CT".to_string(),
                symbol: "LZR-CT".to_string(),
                icon: Some(DATA_IMAGE_SVG_LZR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: TOKEN_DECIMAL as u8,
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
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

    pub fn lzr_locked_in(self) -> U128 {
        self.lzr_locked.into()
    }

    // should only be called after tokens have been transfered to contract
    #[private]
    pub fn ft_mint(&mut self, amount: U128, account_id: AccountId) {
        let amount: Balance = amount.into();
        require!(amount > 0, "Must send loozr to buy tokens");
        let amount_in_near = Decimal::from_i128_with_scale(amount as i128, TOKEN_DECIMAL);
        self.continous_mint(amount_in_near, amount, account_id);
    }

    #[payable]
    pub fn ft_burn(&mut self, sell_amount: U128) {
        assert_one_yocto();
        let sell_amount: Balance = sell_amount.into();
        require!(sell_amount > 0, "Amount must be non-zero.");

        let balance = self.internal_unwrap_balance_of(env::predecessor_account_id());
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
        );
        ext_ft_transfer::ext(get_lzr_contract())
            .with_attached_deposit(1)
            .ft_transfer(env::predecessor_account_id(), reimburse_amount.into())
            .then(Self::ext(env::current_account_id()).on_transfer_callback(
                sell_amount.into(),
                reimburse_amount.into(),
                env::predecessor_account_id().into(),
                env::attached_deposit().into(),
            ));
    }

    #[private]
    pub fn on_transfer_callback(
        &mut self,
        #[callback_result] call_result: Result<(), near_sdk::PromiseError>,
        sell_amount: U128,
        reimburse_amount: U128,
        account_id: AccountId,
        attached_deposit: U128,
    ) {
        // Return whether or not the promise succeeded using the method outlined in external.rs
        if call_result.is_err() {
            let predecessor_account_id = account_id.clone();
            self.internal_mint(sell_amount.into(), account_id);
            self.lzr_locked = self
                .lzr_locked
                .checked_add(reimburse_amount.into())
                .unwrap_or_else(|| env::panic_str("Reserve balance overflow"));

            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
        }
    }

    fn continous_sale(
        &mut self,
        current_supply_in_near: Decimal,
        lzr_locked_in_near: Decimal,
        amount_in_near: Decimal,
        sell_amount: u128,
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
        self.internal_burn(sell_amount);

        reimburse_amount
    }

    fn continous_mint(
        &mut self,
        _deposit: Decimal,
        _amount_in_yocto_near: u128,
        account_id: AccountId,
    ) {
        let amount = self.calc_purchase_return(_deposit);

        self.lzr_locked = self
            .lzr_locked
            .checked_add(_amount_in_yocto_near)
            .unwrap_or_else(|| env::panic_str("Reserve balance overflow"));
        self.internal_mint(amount, account_id);
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

    fn calc_purchase_return(&mut self, _deposit: Decimal) -> u128 {
        if self.lzr_locked == 0 {
            return self.calc_mint_polynomial(
                _deposit,
                self.token.total_supply,
                RESERVE_RATIO,
                Decimal::from_f64(SLOPE).unwrap(),
            );
        }

        return self.calc_mint_bancor(_deposit, self.token.total_supply);
    }

    fn calc_mint_polynomial(
        &self,
        amount: Decimal,
        current_supply: u128,
        reserve_ratio: f64,
        slope: Decimal,
    ) -> u128 {
        let current_supply_in_near =
            Decimal::from_i128_with_scale(current_supply as i128, TOKEN_DECIMAL);
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
            .decimal_to_float(current_supply_in_near)
            .powf(increase_rate as f64) as f64))
            .powf(reserve_ratio))
            - self.decimal_to_float(current_supply_in_near);

        return (result * BASE.pow(TOKEN_DECIMAL) as f64) as u128;
    }

    fn calc_mint_bancor(&self, amount: Decimal, current_supply: u128) -> u128 {
        let current_supply_in_near =
            Decimal::from_i128_with_scale(current_supply as i128, TOKEN_DECIMAL);
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
        result = current_supply_in_near
            * Decimal::from_f64((1. + self.decimal_to_float(result)).powf(RESERVE_RATIO) - 1.)
                .unwrap();

        return (self.decimal_to_float(result) * BASE.pow(TOKEN_DECIMAL) as f64) as u128;
    }

    fn internal_mint(&mut self, amount: Balance, account_id: AccountId) {
        let balance = self.internal_unwrap_balance_of(account_id);
        self.internal_update_account(&env::predecessor_account_id(), balance + amount);
        self.token.total_supply = self
            .token
            .total_supply
            .checked_add(amount)
            .unwrap_or_else(|| env::panic_str("Total supply overflow"));
    }

    fn internal_burn(&mut self, amount: Balance) {
        let balance = self.internal_unwrap_balance_of(env::predecessor_account_id());
        assert!(balance >= amount);
        self.internal_update_account(&env::predecessor_account_id(), balance - amount);
        assert!(self.token.total_supply >= amount);
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
        let mut contract = Contract::new_default_meta(accounts(1).into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(1))
            .build());

        if contract.ft_total_supply().0 != 0 {
            env::panic_str("INCORRECT SUPPLY");
        }
        contract.ft_mint(10000000000000000000000000.into(), accounts(1));
        let balance = contract.ft_total_supply();
        let creator_token_minted: u128 = 21544346900318829112459264;

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
        _contract.continous_mint(
            Decimal::from_u128(10).unwrap(),
            9999999999999999000000000,
            accounts(1),
        );
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into());
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

        contract.continous_mint(
            Decimal::from_u128(500).unwrap(),
            500000000000000000000000000,
            accounts(1),
        );
        let transfer_amount = 15000000000000000000000000;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        if contract.ft_balance_of(accounts(1)).0 != transfer_amount {
            env::panic_str("BALANCE DOES NOT MATCH TRANSFER AMOUNT");
        }
    }
}
