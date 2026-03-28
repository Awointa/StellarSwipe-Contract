#![no_std]

use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    Env, MuxedAddress,
};

/// Protocol fee rate: 0.1% = 10 basis points.
const FEE_RATE_BPS: i128 = 10;
const BPS_DENOM: i128 = 10_000;
/// When `trade_amount * FEE_RATE_BPS < BPS_DENOM`, charge at least one stroop.
const MIN_FEE_STROOPS: i128 = 1;

#[contract]
pub struct FeeCollector;

#[contracttype]
#[derive(Clone)]
pub enum StorageKey {
    AccumulatedFees(Address),
    FeeExempt(Address),
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    NonPositiveTrade = 3,
    Overflow = 4,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    #[topic]
    pub payer: Address,
    #[topic]
    pub token: Address,
    pub amount: i128,
    pub trade_amount: i128,
}

#[contractimpl]
impl FeeCollector {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
    }

    pub fn set_fee_exempt(env: Env, account: Address, exempt: bool) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        admin.require_auth();
        if exempt {
            env.storage()
                .persistent()
                .set(&StorageKey::FeeExempt(account), &true);
        } else {
            env.storage().persistent().remove(&StorageKey::FeeExempt(account));
        }
    }

    /// Deducts the protocol fee from `payer` and credits this contract (SAC / SEP-41).
    /// Returns the fee charged in token stroops (0 if `payer` is fee-exempt).
    pub fn collect_fee(env: Env, payer: Address, trade_amount: i128, token: Address) -> i128 {
        payer.require_auth();

        if trade_amount <= 0 {
            panic_with_error!(&env, Error::NonPositiveTrade);
        }

        let fee_exempt: bool = env
            .storage()
            .persistent()
            .get(&StorageKey::FeeExempt(payer.clone()))
            .unwrap_or(false);

        let fee = if fee_exempt {
            0
        } else {
            let product = trade_amount
                .checked_mul(FEE_RATE_BPS)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
            if product < BPS_DENOM {
                MIN_FEE_STROOPS
            } else {
                product / BPS_DENOM
            }
        };

        if fee > 0 {
            let token_client = TokenClient::new(&env, &token);
            let this = env.current_contract_address();
            token_client.transfer(&payer, &MuxedAddress::from(&this), &fee);
            let prev: i128 = env
                .storage()
                .persistent()
                .get(&StorageKey::AccumulatedFees(token.clone()))
                .unwrap_or(0);
            let next = prev
                .checked_add(fee)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
            env.storage()
                .persistent()
                .set(&StorageKey::AccumulatedFees(token.clone()), &next);
        }

        FeeCollected {
            payer: payer.clone(),
            token: token.clone(),
            amount: fee,
            trade_amount,
        }
        .publish(&env);

        fee
    }

    pub fn get_accumulated_fees(env: Env, token: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&StorageKey::AccumulatedFees(token))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};

    fn setup_fee_collector(env: &Env) -> (Address, Address, Address, FeeCollectorClient<'_>) {
        env.mock_all_auths();
        let admin = Address::generate(env);
        let contract_id = env.register(FeeCollector, ());
        let client = FeeCollectorClient::new(env, &contract_id);
        client.initialize(&admin);
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = sac.address();
        let payer = Address::generate(env);
        StellarAssetClient::new(env, &token).mint(&payer, &100_000_000i128);
        (admin, token, payer, client)
    }

    #[test]
    fn normal_trade_charges_exactly_10_bps() {
        let env = Env::default();
        let (_admin, token, payer, client) = setup_fee_collector(&env);
        let trade_amount = 1_000_000i128;
        let fee = client.collect_fee(&payer, &trade_amount, &token);
        assert_eq!(fee, 1_000); // 1_000_000 * 10 / 10_000 = 0.1%
        let contract_id = client.address.clone();
        assert_eq!(
            TokenClient::new(&env, &token).balance(&contract_id),
            1_000
        );
        assert_eq!(client.get_accumulated_fees(&token), 1_000);
    }

    #[test]
    fn dust_trade_charges_minimum_one_stroop() {
        let env = Env::default();
        let (_admin, token, payer, client) = setup_fee_collector(&env);
        let trade_amount = 999i128;
        assert!(trade_amount * FEE_RATE_BPS < BPS_DENOM);
        let fee = client.collect_fee(&payer, &trade_amount, &token);
        assert_eq!(fee, 1);
        let contract_id = client.address.clone();
        assert_eq!(TokenClient::new(&env, &token).balance(&contract_id), 1);
        assert_eq!(client.get_accumulated_fees(&token), 1);
    }

    #[test]
    fn fee_exempt_address_pays_no_fee() {
        let env = Env::default();
        let (_admin, token, payer, client) = setup_fee_collector(&env);
        client.set_fee_exempt(&payer, &true);
        let trade_amount = 1_000_000i128;
        let fee = client.collect_fee(&payer, &trade_amount, &token);
        assert_eq!(fee, 0);
        let contract_id = client.address.clone();
        assert_eq!(TokenClient::new(&env, &token).balance(&contract_id), 0);
        assert_eq!(client.get_accumulated_fees(&token), 0);
    }
}
