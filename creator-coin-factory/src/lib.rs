use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::Serialize;
use near_sdk::{
    assert_self, env, is_promise_success, near_bindgen, AccountId, Balance, Gas, Promise,
};

const CODE: &[u8] = include_bytes!("../../profile-token-contract/res/loozr_creator_token.wasm");

const NO_DEPOSIT: Balance = 0;
const CREATE_CALL_GAS: u64 = 25_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CreatorCoinFactory {}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct CreatorCoinArgs {
    owner_id: AccountId,
}

#[near_bindgen]
impl CreatorCoinFactory {
    #[payable]
    pub fn create(&mut self, owner_id: AccountId, name: String) -> Promise {
        let creator_coin_account_id = format!("{}.{}", name, env::current_account_id());

        Promise::new(creator_coin_account_id.parse().unwrap())
            .create_account()
            .deploy_contract(CODE.to_vec())
            .transfer(env::attached_deposit())
            .add_full_access_key(env::signer_account_pk())
            .function_call(
                "new_default_meta".into(),
                near_sdk::serde_json::to_vec(&CreatorCoinArgs { owner_id }).unwrap(),
                NO_DEPOSIT,
                Gas(CREATE_CALL_GAS),
            )
            .then(Self::ext(env::current_account_id()).on_coin_create(
                env::attached_deposit().into(),
                env::predecessor_account_id(),
            ))
    }

    /// Callback after a creator coin was created.
    /// Returns true if the coin creation succeeded.
    /// Otherwise refunds the attached deposit and returns `false`.
    pub fn on_coin_create(
        &mut self,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool {
        assert_self();

        let coin_created = is_promise_success();
        if coin_created {
            return true;
        }
        Promise::new(predecessor_account_id).transfer(attached_deposit.0);
        return false;
    }
}
