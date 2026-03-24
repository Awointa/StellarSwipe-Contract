use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_oracle_removed(env: &Env, oracle: Address, reason: &str) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("removed")),
        (oracle, reason),
    );
}

pub fn emit_weight_adjusted(
    env: &Env,
    oracle: Address,
    old_weight: u32,
    new_weight: u32,
    reputation: u32,
) {
    env.events().publish(
        (symbol_short!("weight"), symbol_short!("adjusted")),
        (oracle, old_weight, new_weight, reputation),
    );
}

pub fn emit_oracle_slashed(env: &Env, oracle: Address, reason: &str, penalty: u32) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("slashed")),
        (oracle, reason, penalty),
    );
}

pub fn emit_price_submitted(env: &Env, oracle: Address, price: i128) {
    env.events().publish(
        (symbol_short!("price"), symbol_short!("submit")),
        (oracle, price),
    );
}

pub fn emit_consensus_reached(env: &Env, price: i128, num_oracles: u32) {
    env.events().publish(
        (symbol_short!("consensus"), symbol_short!("reached")),
        (price, num_oracles),
    );
}
