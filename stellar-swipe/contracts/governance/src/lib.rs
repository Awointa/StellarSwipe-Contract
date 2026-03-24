#![no_std]
#![allow(clippy::too_many_arguments)]

mod committees;
mod distribution;
mod errors;
mod token;
mod treasury;

#[cfg(test)]
mod test;

use committees::{
    list_committees as list_registered_committees, CommitteeAction, CommitteeElection,
    CommitteeReport, CommitteesState, CrossCommitteeRequest, VoteType,
};
pub use committees::{
    Authority, Committee, CommitteeDecision, CrossCommitteeStatus, DecisionStatus,
    EmergencyActionAuthority, EmergencyActionPayload, GrantApprovalAction, GrantApprovalAuthority,
    ParameterAdjustmentAuthority, PerformanceMetrics, RewardConfigUpdateAction,
    TreasurySpendAction, TreasurySpendAuthority, VetoAuthority, VetoPayload,
};
use distribution::{
    circulating_supply as calculate_circulating_supply, create_vesting_schedule as create_schedule,
    distribution_state as load_distribution_state, get_schedule, initialize_distribution,
    releasable_amount, release_vested_tokens as release_schedule_tokens, update_reward_config,
    DistributionRecipients, DistributionState, VestingCategory, VestingSchedule,
};
pub use errors::GovernanceError;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec,
};
use stellar_swipe_common::Asset;
pub use token::{HolderAnalytics, HolderBalance, TokenMetadata};
pub use treasury::{
    Budget, BudgetReport, RebalanceAction, RecurringPayment, Treasury, TreasuryReport,
    TreasurySpend,
};

const DEFAULT_LIQUIDITY_REWARD_BPS: u32 = 100;
const DEFAULT_MIN_CLAIM_THRESHOLD: i128 = 100;

#[contract]
pub struct GovernanceContract;

#[contracttype]
#[derive(Clone)]
pub enum StorageKey {
    Admin,
    Initialized,
    Metadata,
    Balances,
    StakedBalances,
    PendingRewards,
    VestingSchedules,
    Holders,
    DistributionState,
    VoteLocks,
    Treasury,
    Committees,
}

#[allow(clippy::too_many_arguments)]
#[contractimpl]
impl GovernanceContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        name: String,
        symbol: String,
        decimals: u32,
        total_supply: i128,
        recipients: DistributionRecipients,
    ) -> Result<(), GovernanceError> {
        admin.require_auth();

        if is_initialized(&env) {
            return Err(GovernanceError::AlreadyInitialized);
        }
        if total_supply <= 0 {
            return Err(GovernanceError::InvalidSupply);
        }
        if name.is_empty() || symbol.is_empty() {
            return Err(GovernanceError::InvalidMetadata);
        }

        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage().instance().set(
            &StorageKey::Metadata,
            &TokenMetadata {
                name: name.clone(),
                symbol: symbol.clone(),
                decimals,
                total_supply,
            },
        );
        env.storage()
            .instance()
            .set(&StorageKey::Balances, &Map::<Address, i128>::new(&env));
        env.storage().instance().set(
            &StorageKey::StakedBalances,
            &Map::<Address, i128>::new(&env),
        );
        env.storage().instance().set(
            &StorageKey::PendingRewards,
            &Map::<Address, i128>::new(&env),
        );
        env.storage().instance().set(
            &StorageKey::VestingSchedules,
            &Map::<Address, VestingSchedule>::new(&env),
        );
        env.storage()
            .instance()
            .set(&StorageKey::VoteLocks, &Map::<Address, u32>::new(&env));
        env.storage()
            .instance()
            .set(&StorageKey::Holders, &Vec::<Address>::new(&env));
        env.storage()
            .instance()
            .set(&StorageKey::Treasury, &treasury::empty_treasury(&env));
        env.storage().instance().set(
            &StorageKey::Committees,
            &committees::empty_committees_state(&env),
        );

        let distribution = initialize_distribution(
            &env,
            &recipients,
            total_supply,
            DEFAULT_LIQUIDITY_REWARD_BPS,
            DEFAULT_MIN_CLAIM_THRESHOLD,
        )?;

        env.storage()
            .instance()
            .set(&StorageKey::Initialized, &true);
        track_holder(&env, &recipients.team);
        track_holder(&env, &recipients.early_investors);
        track_holder(&env, &recipients.community_rewards);
        track_holder(&env, &recipients.treasury);
        track_holder(&env, &recipients.public_sale);

        emit_initialized(&env, &admin, &name, &symbol, total_supply);
        emit_distribution_initialized(&env, &distribution);
        Ok(())
    }

    pub fn get_metadata(env: Env) -> Result<TokenMetadata, GovernanceError> {
        require_initialized(&env)?;
        metadata(&env)
    }

    pub fn total_supply(env: Env) -> Result<i128, GovernanceError> {
        get_total_supply(&env)
    }

    pub fn circulating_supply(env: Env) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        calculate_circulating_supply(&env)
    }

    pub fn balance(env: Env, holder: Address) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        Ok(get_balance(&env, &holder))
    }

    pub fn staked_balance(env: Env, holder: Address) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        Ok(get_staked_balance(&env, &holder))
    }

    pub fn voting_power(env: Env, holder: Address) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        Ok(get_staked_balance(&env, &holder))
    }

    pub fn distribution(env: Env) -> Result<DistributionState, GovernanceError> {
        require_initialized(&env)?;
        load_distribution_state(&env)
    }

    pub fn create_vesting_schedule(
        env: Env,
        admin: Address,
        beneficiary: Address,
        total_amount: i128,
        start_time: u64,
        cliff_seconds: u64,
        duration_seconds: u64,
    ) -> Result<(), GovernanceError> {
        require_admin(&env, &admin)?;
        create_schedule(
            &env,
            beneficiary.clone(),
            VestingCategory::Custom,
            total_amount,
            start_time,
            cliff_seconds,
            duration_seconds,
        )?;
        track_holder(&env, &beneficiary);
        emit_vesting_created(
            &env,
            &beneficiary,
            total_amount,
            cliff_seconds,
            duration_seconds,
        );
        Ok(())
    }

    pub fn get_vesting_schedule(
        env: Env,
        beneficiary: Address,
    ) -> Result<VestingSchedule, GovernanceError> {
        require_initialized(&env)?;
        get_schedule(&env, &beneficiary)
    }

    pub fn releasable_vested_amount(
        env: Env,
        beneficiary: Address,
    ) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        releasable_amount(&env, &beneficiary)
    }

    pub fn release_vested_tokens(env: Env, beneficiary: Address) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        beneficiary.require_auth();
        let (_, amount) = release_schedule_tokens(&env, &beneficiary)?;
        emit_vesting_released(&env, &beneficiary, amount);
        Ok(amount)
    }

    pub fn stake(env: Env, user: Address, amount: i128) -> Result<(), GovernanceError> {
        require_initialized(&env)?;
        user.require_auth();
        token::stake(&env, &user, amount)?;
        emit_stake_changed(&env, &user, amount, true);
        Ok(())
    }

    pub fn unstake(env: Env, user: Address, amount: i128) -> Result<(), GovernanceError> {
        require_initialized(&env)?;
        user.require_auth();
        token::unstake(&env, &user, amount)?;
        emit_stake_changed(&env, &user, amount, false);
        Ok(())
    }

    pub fn set_vote_lock(
        env: Env,
        admin: Address,
        holder: Address,
        active_votes: u32,
    ) -> Result<(), GovernanceError> {
        require_admin(&env, &admin)?;
        token::set_vote_lock(&env, &holder, active_votes)?;
        emit_admin_action(
            &env,
            symbol_short!("votelock"),
            &holder,
            active_votes as i128,
        );
        Ok(())
    }

    pub fn accrue_liquidity_rewards(
        env: Env,
        admin: Address,
        beneficiary: Address,
        trading_volume: i128,
    ) -> Result<i128, GovernanceError> {
        require_admin(&env, &admin)?;
        let reward = token::accrue_liquidity_rewards(&env, &beneficiary, trading_volume)?;
        emit_reward_accrued(&env, &beneficiary, trading_volume, reward);
        Ok(reward)
    }

    pub fn claim_liquidity_rewards(
        env: Env,
        beneficiary: Address,
    ) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        beneficiary.require_auth();
        let amount = token::claim_liquidity_rewards(&env, &beneficiary)?;
        emit_reward_claimed(&env, &beneficiary, amount);
        Ok(amount)
    }

    pub fn pending_rewards(env: Env, beneficiary: Address) -> Result<i128, GovernanceError> {
        require_initialized(&env)?;
        Ok(get_pending_rewards(&env).get(beneficiary).unwrap_or(0))
    }

    pub fn set_liquidity_mining_config(
        env: Env,
        admin: Address,
        reward_bps: u32,
        min_claim_threshold: i128,
    ) -> Result<DistributionState, GovernanceError> {
        require_admin(&env, &admin)?;
        let state = update_reward_config(&env, reward_bps, min_claim_threshold)?;
        emit_admin_action(&env, symbol_short!("rewardcfg"), &admin, reward_bps as i128);
        Ok(state)
    }

    pub fn analytics(env: Env, top_n: u32) -> Result<HolderAnalytics, GovernanceError> {
        token::analytics(&env, top_n)
    }

    pub fn treasury(env: Env) -> Result<Treasury, GovernanceError> {
        require_initialized(&env)?;
        Ok(get_treasury(&env))
    }

    pub fn set_treasury_asset(
        env: Env,
        admin: Address,
        asset: Asset,
        amount: i128,
    ) -> Result<Treasury, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        treasury::set_asset_balance(&env, &mut treasury, asset, amount)?;
        put_treasury(&env, &treasury);
        emit_admin_action(&env, symbol_short!("trsasset"), &admin, amount);
        Ok(treasury)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_budget(
        env: Env,
        admin: Address,
        category: String,
        allocated: i128,
        spend_limit: i128,
        period_start: u64,
        period_end: u64,
        auto_renew: bool,
    ) -> Result<Budget, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        let budget = treasury::upsert_budget(
            &env,
            &mut treasury,
            category,
            allocated,
            spend_limit,
            period_start,
            period_end,
            auto_renew,
        )?;
        put_treasury(&env, &treasury);
        emit_admin_action(&env, symbol_short!("budget"), &admin, allocated);
        Ok(budget)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_treasury_spend(
        env: Env,
        admin: Address,
        recipient: Address,
        amount: i128,
        asset: Asset,
        category: String,
        purpose: String,
        approved_by_proposal: Option<u64>,
    ) -> Result<TreasurySpend, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        let spend = treasury::execute_spend(
            &mut treasury,
            recipient,
            amount,
            asset,
            category,
            purpose,
            approved_by_proposal,
            env.ledger().timestamp(),
        )?;
        put_treasury(&env, &treasury);
        emit_admin_action(&env, symbol_short!("spend"), &admin, spend.amount);
        Ok(spend)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_recurring_payment(
        env: Env,
        admin: Address,
        recipient: Address,
        amount: i128,
        asset: Asset,
        frequency: u64,
        category: String,
        purpose: String,
        approved_by_proposal: Option<u64>,
        end_date: Option<u64>,
    ) -> Result<RecurringPayment, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        let payment = treasury::schedule_recurring_payment(
            &env,
            &mut treasury,
            recipient,
            amount,
            asset,
            frequency,
            category,
            purpose,
            approved_by_proposal,
            end_date,
        )?;
        put_treasury(&env, &treasury);
        emit_admin_action(&env, symbol_short!("recur"), &admin, amount);
        Ok(payment)
    }

    pub fn process_recurring_payments(env: Env, admin: Address) -> Result<u32, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        let processed =
            treasury::process_recurring_payments(&mut treasury, env.ledger().timestamp())?;
        put_treasury(&env, &treasury);
        emit_admin_action(&env, symbol_short!("payrun"), &admin, processed as i128);
        Ok(processed)
    }

    pub fn treasury_report(env: Env) -> Result<TreasuryReport, GovernanceError> {
        require_initialized(&env)?;
        treasury::build_report(&env, &get_treasury(&env))
    }

    pub fn committees(env: Env) -> Result<Vec<Committee>, GovernanceError> {
        require_initialized(&env)?;
        Ok(list_registered_committees(
            &env,
            &get_committees_state(&env),
        ))
    }

    pub fn committee(env: Env, committee_id: u64) -> Result<Committee, GovernanceError> {
        require_initialized(&env)?;
        committees::get_committee(&get_committees_state(&env), committee_id)
    }

    pub fn create_committee(
        env: Env,
        admin: Address,
        name: String,
        description: String,
        initial_members: Vec<Address>,
        chair: Address,
        max_members: u32,
        authorities: Vec<Authority>,
        term_duration_days: Option<u32>,
    ) -> Result<Committee, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut committees_state = get_committees_state(&env);
        let committee = committees::create_committee(
            &env,
            &mut committees_state,
            name,
            description,
            initial_members,
            chair,
            max_members,
            authorities,
            term_duration_days,
        )?;
        put_committees_state(&env, &committees_state);
        emit_admin_action(&env, symbol_short!("cmtadd"), &admin, committee.id as i128);
        Ok(committee)
    }

    pub fn propose_committee_decision(
        env: Env,
        committee_id: u64,
        proposer: Address,
        proposal: String,
        action: CommitteeAction,
    ) -> Result<CommitteeDecision, GovernanceError> {
        require_initialized(&env)?;
        proposer.require_auth();
        let mut committees_state = get_committees_state(&env);
        let decision = committees::propose_decision(
            &env,
            &mut committees_state,
            committee_id,
            proposer,
            proposal,
            action,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(decision)
    }

    pub fn vote_on_committee_decision(
        env: Env,
        committee_id: u64,
        decision_id: u64,
        voter: Address,
        vote: VoteType,
    ) -> Result<CommitteeDecision, GovernanceError> {
        require_initialized(&env)?;
        voter.require_auth();
        let mut committees_state = get_committees_state(&env);
        let decision = committees::vote_on_decision(
            &mut committees_state,
            committee_id,
            decision_id,
            voter,
            vote,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(decision)
    }

    pub fn execute_committee_decision(
        env: Env,
        committee_id: u64,
        decision_id: u64,
        executor: Address,
    ) -> Result<CommitteeDecision, GovernanceError> {
        require_initialized(&env)?;
        executor.require_auth();
        let mut committees_state = get_committees_state(&env);
        let decision = committees::execute_decision(
            &env,
            &mut committees_state,
            committee_id,
            decision_id,
            executor,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(decision)
    }

    pub fn start_committee_election(
        env: Env,
        admin: Address,
        committee_id: u64,
        positions_available: u32,
        duration_days: u32,
    ) -> Result<CommitteeElection, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut committees_state = get_committees_state(&env);
        let election = committees::start_election(
            &env,
            &mut committees_state,
            committee_id,
            positions_available,
            duration_days,
        )?;
        put_committees_state(&env, &committees_state);
        emit_admin_action(
            &env,
            symbol_short!("cmtelect"),
            &admin,
            committee_id as i128,
        );
        Ok(election)
    }

    pub fn committee_election(
        env: Env,
        committee_id: u64,
    ) -> Result<CommitteeElection, GovernanceError> {
        require_initialized(&env)?;
        committees::get_election(&get_committees_state(&env), committee_id)
    }

    pub fn nominate_for_committee(
        env: Env,
        committee_id: u64,
        nominee: Address,
        nominator: Address,
    ) -> Result<CommitteeElection, GovernanceError> {
        require_initialized(&env)?;
        nominee.require_auth();
        nominator.require_auth();
        let mut committees_state = get_committees_state(&env);
        let election = committees::nominate_for_committee(
            &env,
            &mut committees_state,
            committee_id,
            nominee,
            nominator,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(election)
    }

    pub fn vote_in_committee_election(
        env: Env,
        committee_id: u64,
        voter: Address,
        candidate: Address,
    ) -> Result<CommitteeElection, GovernanceError> {
        require_initialized(&env)?;
        voter.require_auth();
        let mut committees_state = get_committees_state(&env);
        let election = committees::vote_in_election(
            &env,
            &mut committees_state,
            committee_id,
            voter,
            candidate,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(election)
    }

    pub fn finalize_committee_election(
        env: Env,
        admin: Address,
        committee_id: u64,
    ) -> Result<Vec<Address>, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut committees_state = get_committees_state(&env);
        let winners = committees::finalize_election(&env, &mut committees_state, committee_id)?;
        put_committees_state(&env, &committees_state);
        emit_admin_action(
            &env,
            symbol_short!("cmtfinal"),
            &admin,
            committee_id as i128,
        );
        Ok(winners)
    }

    pub fn set_committee_approval_rating(
        env: Env,
        admin: Address,
        committee_id: u64,
        community_approval_rating: u32,
    ) -> Result<Committee, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut committees_state = get_committees_state(&env);
        let committee = committees::set_community_approval_rating(
            &mut committees_state,
            committee_id,
            community_approval_rating,
        )?;
        put_committees_state(&env, &committees_state);
        emit_admin_action(
            &env,
            symbol_short!("cmtrank"),
            &admin,
            community_approval_rating as i128,
        );
        Ok(committee)
    }

    pub fn committee_report(
        env: Env,
        committee_id: u64,
    ) -> Result<CommitteeReport, GovernanceError> {
        require_initialized(&env)?;
        committees::report_activity(&env, &get_committees_state(&env), committee_id)
    }

    pub fn override_committee_decision(
        env: Env,
        admin: Address,
        committee_id: u64,
        decision_id: u64,
    ) -> Result<CommitteeDecision, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut committees_state = get_committees_state(&env);
        let decision =
            committees::override_decision(&mut committees_state, committee_id, decision_id)?;
        put_committees_state(&env, &committees_state);
        emit_admin_action(&env, symbol_short!("cmtover"), &admin, decision_id as i128);
        Ok(decision)
    }

    pub fn dissolve_committee(
        env: Env,
        admin: Address,
        committee_id: u64,
    ) -> Result<Committee, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut committees_state = get_committees_state(&env);
        let committee = committees::dissolve_committee(&env, &mut committees_state, committee_id)?;
        put_committees_state(&env, &committees_state);
        emit_admin_action(&env, symbol_short!("cmtdrop"), &admin, committee_id as i128);
        Ok(committee)
    }

    pub fn request_cross_committee_approval(
        env: Env,
        requesting_committee: u64,
        requester: Address,
        approving_committees: Vec<u64>,
        proposal: String,
    ) -> Result<CrossCommitteeRequest, GovernanceError> {
        require_initialized(&env)?;
        requester.require_auth();
        let mut committees_state = get_committees_state(&env);
        let request = committees::request_cross_committee_approval(
            &env,
            &mut committees_state,
            requesting_committee,
            requester,
            approving_committees,
            proposal,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(request)
    }

    pub fn approve_cross_committee_request(
        env: Env,
        request_id: u64,
        approving_committee: u64,
        approver: Address,
        decision_id: u64,
    ) -> Result<CrossCommitteeRequest, GovernanceError> {
        require_initialized(&env)?;
        approver.require_auth();
        let mut committees_state = get_committees_state(&env);
        let request = committees::approve_cross_committee_request(
            &mut committees_state,
            request_id,
            approving_committee,
            approver,
            decision_id,
        )?;
        put_committees_state(&env, &committees_state);
        Ok(request)
    }

    pub fn cross_committee_request(
        env: Env,
        request_id: u64,
    ) -> Result<CrossCommitteeRequest, GovernanceError> {
        require_initialized(&env)?;
        committees::get_cross_committee_request(&get_committees_state(&env), request_id)
    }

    pub fn set_rebalance_target(
        env: Env,
        admin: Address,
        asset: Asset,
        target_bps: i128,
    ) -> Result<Treasury, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        treasury::set_rebalance_target(&env, &mut treasury, asset, target_bps)?;
        put_treasury(&env, &treasury);
        emit_admin_action(&env, symbol_short!("target"), &admin, target_bps);
        Ok(treasury)
    }

    pub fn rebalance_treasury(
        env: Env,
        admin: Address,
        prices: Map<Asset, i128>,
    ) -> Result<Vec<RebalanceAction>, GovernanceError> {
        require_admin(&env, &admin)?;
        let mut treasury = get_treasury(&env);
        let actions = treasury::rebalance(&mut treasury, prices, env.ledger().timestamp(), &env)?;
        put_treasury(&env, &treasury);
        emit_admin_action(
            &env,
            symbol_short!("rebalance"),
            &admin,
            treasury.total_value_usd,
        );
        Ok(actions)
    }
}

fn is_initialized(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&StorageKey::Initialized)
        .unwrap_or(false)
}

fn metadata(env: &Env) -> Result<TokenMetadata, GovernanceError> {
    env.storage()
        .instance()
        .get(&StorageKey::Metadata)
        .ok_or(GovernanceError::NotInitialized)
}

pub(crate) fn get_total_supply(env: &Env) -> Result<i128, GovernanceError> {
    Ok(metadata(env)?.total_supply)
}

pub(crate) fn require_initialized(env: &Env) -> Result<(), GovernanceError> {
    if is_initialized(env) {
        Ok(())
    } else {
        Err(GovernanceError::NotInitialized)
    }
}

fn require_admin(env: &Env, caller: &Address) -> Result<(), GovernanceError> {
    require_initialized(env)?;
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&StorageKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;
    if admin != *caller {
        return Err(GovernanceError::Unauthorized);
    }
    Ok(())
}

fn balances(env: &Env) -> Map<Address, i128> {
    env.storage()
        .instance()
        .get(&StorageKey::Balances)
        .unwrap_or(Map::new(env))
}

fn put_balances(env: &Env, balances: &Map<Address, i128>) {
    env.storage()
        .instance()
        .set(&StorageKey::Balances, balances);
}

pub(crate) fn get_balance(env: &Env, holder: &Address) -> i128 {
    balances(env).get(holder.clone()).unwrap_or(0)
}

pub(crate) fn add_balance(
    env: &Env,
    holder: &Address,
    amount: i128,
) -> Result<(), GovernanceError> {
    if amount <= 0 {
        return Err(GovernanceError::InvalidAmount);
    }
    let mut map = balances(env);
    let current = map.get(holder.clone()).unwrap_or(0);
    map.set(holder.clone(), checked_add(current, amount)?);
    put_balances(env, &map);
    track_holder(env, holder);
    Ok(())
}

pub(crate) fn subtract_balance(
    env: &Env,
    holder: &Address,
    amount: i128,
) -> Result<(), GovernanceError> {
    if amount <= 0 {
        return Err(GovernanceError::InvalidAmount);
    }
    let mut map = balances(env);
    let current = map.get(holder.clone()).unwrap_or(0);
    if current < amount {
        return Err(GovernanceError::InsufficientBalance);
    }
    map.set(holder.clone(), checked_sub(current, amount)?);
    put_balances(env, &map);
    track_holder(env, holder);
    Ok(())
}

fn staked_balances(env: &Env) -> Map<Address, i128> {
    env.storage()
        .instance()
        .get(&StorageKey::StakedBalances)
        .unwrap_or(Map::new(env))
}

fn put_staked_balances(env: &Env, staked: &Map<Address, i128>) {
    env.storage()
        .instance()
        .set(&StorageKey::StakedBalances, staked);
}

pub(crate) fn get_staked_balance(env: &Env, holder: &Address) -> i128 {
    staked_balances(env).get(holder.clone()).unwrap_or(0)
}

pub(crate) fn add_staked_balance(
    env: &Env,
    holder: &Address,
    amount: i128,
) -> Result<(), GovernanceError> {
    if amount <= 0 {
        return Err(GovernanceError::InvalidAmount);
    }
    let mut map = staked_balances(env);
    let current = map.get(holder.clone()).unwrap_or(0);
    map.set(holder.clone(), checked_add(current, amount)?);
    put_staked_balances(env, &map);
    track_holder(env, holder);
    Ok(())
}

pub(crate) fn subtract_staked_balance(
    env: &Env,
    holder: &Address,
    amount: i128,
) -> Result<(), GovernanceError> {
    if amount <= 0 {
        return Err(GovernanceError::InvalidAmount);
    }
    let mut map = staked_balances(env);
    let current = map.get(holder.clone()).unwrap_or(0);
    if current < amount {
        return Err(GovernanceError::InsufficientStakedBalance);
    }
    map.set(holder.clone(), checked_sub(current, amount)?);
    put_staked_balances(env, &map);
    track_holder(env, holder);
    Ok(())
}

pub(crate) fn get_pending_rewards(env: &Env) -> Map<Address, i128> {
    env.storage()
        .instance()
        .get(&StorageKey::PendingRewards)
        .unwrap_or(Map::new(env))
}

pub(crate) fn put_pending_rewards(env: &Env, rewards: &Map<Address, i128>) {
    env.storage()
        .instance()
        .set(&StorageKey::PendingRewards, rewards);
}

pub(crate) fn get_vesting_schedules(env: &Env) -> Map<Address, VestingSchedule> {
    env.storage()
        .instance()
        .get(&StorageKey::VestingSchedules)
        .unwrap_or(Map::new(env))
}

pub(crate) fn put_vesting_schedules(env: &Env, schedules: &Map<Address, VestingSchedule>) {
    env.storage()
        .instance()
        .set(&StorageKey::VestingSchedules, schedules);
}

pub(crate) fn get_distribution_state(env: &Env) -> Result<DistributionState, GovernanceError> {
    env.storage()
        .instance()
        .get(&StorageKey::DistributionState)
        .ok_or(GovernanceError::NotInitialized)
}

pub(crate) fn put_distribution_state(env: &Env, state: &DistributionState) {
    env.storage()
        .instance()
        .set(&StorageKey::DistributionState, state);
}

pub(crate) fn get_vote_locks(env: &Env) -> Map<Address, u32> {
    env.storage()
        .instance()
        .get(&StorageKey::VoteLocks)
        .unwrap_or(Map::new(env))
}

pub(crate) fn get_treasury(env: &Env) -> Treasury {
    env.storage()
        .instance()
        .get(&StorageKey::Treasury)
        .unwrap_or_else(|| treasury::empty_treasury(env))
}

pub(crate) fn put_treasury(env: &Env, treasury_state: &Treasury) {
    env.storage()
        .instance()
        .set(&StorageKey::Treasury, treasury_state);
}

pub(crate) fn get_committees_state(env: &Env) -> CommitteesState {
    env.storage()
        .instance()
        .get(&StorageKey::Committees)
        .unwrap_or_else(|| committees::empty_committees_state(env))
}

pub(crate) fn put_committees_state(env: &Env, committees_state: &CommitteesState) {
    env.storage()
        .instance()
        .set(&StorageKey::Committees, committees_state);
}

pub(crate) fn put_vote_locks(env: &Env, locks: &Map<Address, u32>) {
    env.storage().instance().set(&StorageKey::VoteLocks, locks);
}

pub(crate) fn get_holders(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&StorageKey::Holders)
        .unwrap_or(Vec::new(env))
}

fn put_holders(env: &Env, holders: &Vec<Address>) {
    env.storage().instance().set(&StorageKey::Holders, holders);
}

pub(crate) fn track_holder(env: &Env, holder: &Address) {
    let mut holders = get_holders(env);
    let mut index = 0;
    while index < holders.len() {
        if holders.get(index).unwrap() == *holder {
            return;
        }
        index += 1;
    }
    holders.push_back(holder.clone());
    put_holders(env, &holders);
}

pub(crate) fn checked_add(left: i128, right: i128) -> Result<i128, GovernanceError> {
    left.checked_add(right)
        .ok_or(GovernanceError::ArithmeticOverflow)
}

pub(crate) fn checked_sub(left: i128, right: i128) -> Result<i128, GovernanceError> {
    left.checked_sub(right)
        .ok_or(GovernanceError::ArithmeticOverflow)
}

pub(crate) fn checked_mul(left: i128, right: i128) -> Result<i128, GovernanceError> {
    left.checked_mul(right)
        .ok_or(GovernanceError::ArithmeticOverflow)
}

pub(crate) fn checked_div(left: i128, right: i128) -> Result<i128, GovernanceError> {
    left.checked_div(right)
        .ok_or(GovernanceError::ArithmeticOverflow)
}

#[allow(deprecated)]
fn emit_initialized(
    env: &Env,
    admin: &Address,
    name: &String,
    symbol: &String,
    total_supply: i128,
) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("init")),
        (admin.clone(), name.clone(), symbol.clone(), total_supply),
    );
}

#[allow(deprecated)]
fn emit_distribution_initialized(env: &Env, state: &DistributionState) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("dist")),
        (
            state.allocation.team,
            state.allocation.early_investors,
            state.allocation.community_rewards,
            state.allocation.liquidity_mining,
            state.allocation.treasury,
            state.allocation.public_sale,
        ),
    );
}

#[allow(deprecated)]
fn emit_vesting_created(
    env: &Env,
    beneficiary: &Address,
    amount: i128,
    cliff_seconds: u64,
    duration_seconds: u64,
) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("vestadd")),
        (
            beneficiary.clone(),
            amount,
            cliff_seconds as i128,
            duration_seconds as i128,
        ),
    );
}

#[allow(deprecated)]
fn emit_vesting_released(env: &Env, beneficiary: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("vestrel")),
        (beneficiary.clone(), amount),
    );
}

#[allow(deprecated)]
fn emit_stake_changed(env: &Env, holder: &Address, amount: i128, is_stake: bool) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("stake")),
        (holder.clone(), amount, is_stake),
    );
}

#[allow(deprecated)]
fn emit_reward_accrued(env: &Env, beneficiary: &Address, volume: i128, reward: i128) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("accrue")),
        (beneficiary.clone(), volume, reward),
    );
}

#[allow(deprecated)]
fn emit_reward_claimed(env: &Env, beneficiary: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("claim")),
        (beneficiary.clone(), amount),
    );
}

#[allow(deprecated)]
fn emit_admin_action(env: &Env, action: Symbol, actor: &Address, value: i128) {
    env.events()
        .publish((symbol_short!("gov"), action), (actor.clone(), value));
}
