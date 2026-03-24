extern crate std;

use crate::distribution::{
    DistributionRecipients, EARLY_INVESTOR_VESTING_DURATION, TEAM_CLIFF_DURATION,
    TEAM_VESTING_DURATION, YEAR_SECONDS,
};
use crate::{GovernanceContract, GovernanceContractClient, GovernanceError};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, String};

const SUPPLY: i128 = 1_000_000_000;

fn setup() -> (Env, Address, Address, DistributionRecipients) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(0);

    let contract_id = env.register(GovernanceContract, ());
    let admin = Address::generate(&env);
    let recipients = DistributionRecipients {
        team: Address::generate(&env),
        early_investors: Address::generate(&env),
        community_rewards: Address::generate(&env),
        treasury: Address::generate(&env),
        public_sale: Address::generate(&env),
    };

    (env, contract_id, admin, recipients)
}

fn client<'a>(env: &'a Env, contract_id: &'a Address) -> GovernanceContractClient<'a> {
    GovernanceContractClient::new(env, contract_id)
}

fn initialize(
    client: &GovernanceContractClient<'_>,
    env: &Env,
    admin: &Address,
    recipients: &DistributionRecipients,
) {
    client.initialize(
        admin,
        &String::from_str(env, "StellarSwipe Gov"),
        &String::from_str(env, "SSG"),
        &7u32,
        &SUPPLY,
        recipients,
    );
}

#[test]
fn initialize_governance_token_with_valid_total_supply() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let metadata = client.get_metadata();
    assert_eq!(metadata.total_supply, SUPPLY);
    assert_eq!(metadata.decimals, 7);
}

#[test]
fn reject_zero_invalid_total_supply() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);

    let result = client.try_initialize(
        &admin,
        &String::from_str(&env, "StellarSwipe Gov"),
        &String::from_str(&env, "SSG"),
        &7u32,
        &0i128,
        &recipients,
    );
    assert_eq!(result, Err(Ok(GovernanceError::InvalidSupply)));
}

#[test]
fn allocate_initial_distribution_correctly_from_one_billion_supply() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let distribution = client.distribution();
    assert_eq!(distribution.allocation.team, 200_000_000);
    assert_eq!(distribution.allocation.early_investors, 150_000_000);
    assert_eq!(distribution.allocation.community_rewards, 300_000_000);
    assert_eq!(distribution.allocation.liquidity_mining, 200_000_000);
    assert_eq!(distribution.allocation.treasury, 100_000_000);
    assert_eq!(distribution.allocation.public_sale, 50_000_000);
    assert_eq!(client.balance(&recipients.community_rewards), 300_000_000);
    assert_eq!(client.balance(&recipients.treasury), 100_000_000);
    assert_eq!(client.balance(&recipients.public_sale), 50_000_000);
}

#[test]
fn create_team_vesting_schedule() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let schedule = client.get_vesting_schedule(&recipients.team);
    assert_eq!(schedule.total_amount, 200_000_000);
    assert_eq!(schedule.cliff_seconds, TEAM_CLIFF_DURATION);
    assert_eq!(schedule.duration_seconds, TEAM_VESTING_DURATION);
}

#[test]
fn enforce_cliff_before_release() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);
    env.ledger().set_timestamp(TEAM_CLIFF_DURATION - 1);

    let result = client.try_release_vested_tokens(&recipients.team);
    assert_eq!(result, Err(Ok(GovernanceError::CliffNotReached)));
}

#[test]
fn release_vested_tokens_after_cliff_over_time() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);
    env.ledger()
        .set_timestamp(TEAM_CLIFF_DURATION + (YEAR_SECONDS / 2));

    let released = client.release_vested_tokens(&recipients.team);
    assert_eq!(released, 33_333_333);
    assert_eq!(client.balance(&recipients.team), released);
}

#[test]
fn full_vesting_release_at_end_of_duration() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);
    env.ledger().set_timestamp(TEAM_VESTING_DURATION);

    let released = client.release_vested_tokens(&recipients.team);
    assert_eq!(released, 200_000_000);
    assert_eq!(client.balance(&recipients.team), 200_000_000);
}

#[test]
fn stake_tokens_updates_balances_and_voting_power() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    client.stake(&recipients.community_rewards, &50_000_000);
    assert_eq!(client.balance(&recipients.community_rewards), 250_000_000);
    assert_eq!(
        client.staked_balance(&recipients.community_rewards),
        50_000_000
    );
    assert_eq!(
        client.voting_power(&recipients.community_rewards),
        50_000_000
    );
}

#[test]
fn unstake_fails_with_insufficient_staked_balance() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let result = client.try_unstake(&recipients.community_rewards, &1i128);
    assert_eq!(result, Err(Ok(GovernanceError::InsufficientStakedBalance)));
}

#[test]
fn accrue_liquidity_mining_rewards() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let reward = client.accrue_liquidity_rewards(&admin, &recipients.public_sale, &50_000);
    assert_eq!(reward, 500);
    assert_eq!(client.pending_rewards(&recipients.public_sale), 500);
}

#[test]
fn claim_liquidity_mining_rewards() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    client.accrue_liquidity_rewards(&admin, &recipients.public_sale, &50_000);
    let claimed = client.claim_liquidity_rewards(&recipients.public_sale);
    assert_eq!(claimed, 500);
    assert_eq!(client.pending_rewards(&recipients.public_sale), 0);
    assert_eq!(client.balance(&recipients.public_sale), 50_000_500);
}

#[test]
fn analytics_returns_sane_stats() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);
    client.stake(&recipients.community_rewards, &100_000_000);
    client.accrue_liquidity_rewards(&admin, &recipients.public_sale, &100_000);
    client.claim_liquidity_rewards(&recipients.public_sale);

    let analytics = client.analytics(&3);
    assert_eq!(analytics.total_holders, 3);
    assert_eq!(analytics.total_staked, 100_000_000);
    assert!(analytics.staking_ratio_bps > 0);
    assert_eq!(analytics.top_holders.len(), 3);
}

#[test]
fn edge_cases_duplicate_schedules_zero_amount_and_over_claim_are_covered() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let duplicate =
        client.try_create_vesting_schedule(&admin, &recipients.team, &10i128, &0u64, &0u64, &10u64);
    assert_eq!(duplicate, Err(Ok(GovernanceError::DuplicateSchedule)));

    let zero_amount = client.try_stake(&recipients.community_rewards, &0i128);
    assert_eq!(zero_amount, Err(Ok(GovernanceError::InvalidAmount)));

    let reward = client.accrue_liquidity_rewards(&admin, &recipients.public_sale, &1_000);
    assert_eq!(reward, 10);
    let below_threshold = client.try_claim_liquidity_rewards(&recipients.public_sale);
    assert_eq!(below_threshold, Err(Ok(GovernanceError::BelowMinimumClaim)));

    env.ledger().set_timestamp(TEAM_CLIFF_DURATION + 1);
    let first_release = client.release_vested_tokens(&recipients.team);
    assert!(first_release > 0);
    let second_release = client.try_release_vested_tokens(&recipients.team);
    assert_eq!(second_release, Err(Ok(GovernanceError::NothingToRelease)));
}

#[test]
fn early_investor_vesting_releases_fully_at_end() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);

    let schedule = client.get_vesting_schedule(&recipients.early_investors);
    assert_eq!(schedule.duration_seconds, EARLY_INVESTOR_VESTING_DURATION);

    env.ledger().set_timestamp(EARLY_INVESTOR_VESTING_DURATION);
    let released = client.release_vested_tokens(&recipients.early_investors);
    assert_eq!(released, 150_000_000);
}

#[test]
fn active_vote_lock_blocks_unstake() {
    let (env, contract_id, admin, recipients) = setup();
    let client = client(&env, &contract_id);
    initialize(&client, &env, &admin, &recipients);
    client.stake(&recipients.community_rewards, &10_000);
    client.set_vote_lock(&admin, &recipients.community_rewards, &1);

    let result = client.try_unstake(&recipients.community_rewards, &1_000);
    assert_eq!(result, Err(Ok(GovernanceError::ActiveVoteLock)));
}
