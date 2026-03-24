# Oracle Integration Guide

## Integrating Oracle with Auto Trade Contract

This guide shows how to use the Oracle contract for portfolio valuation and multi-asset aggregation.

## Step 1: Add Oracle Dependency

Update `contracts/auto_trade/Cargo.toml`:

```toml
[dependencies]
soroban-sdk = { workspace = true }
common = { path = "../common" }
oracle = { path = "../oracle" }  # Add this line
```

## Step 2: Import Oracle Functions

In `contracts/auto_trade/src/portfolio.rs`:

```rust
use oracle::{convert_to_base, OracleContractClient};
```

## Step 3: Store Oracle Contract Address

Add to storage.rs:

```rust
#[contracttype]
pub enum StorageKey {
    // ... existing keys
    OracleContract,
}

pub fn set_oracle_contract(env: &Env, contract_id: Address) {
    env.storage().instance().set(&StorageKey::OracleContract, &contract_id);
}

pub fn get_oracle_contract(env: &Env) -> Address {
    env.storage().instance().get(&StorageKey::OracleContract).unwrap()
}
```

## Step 4: Update Portfolio Valuation

Modify `contracts/auto_trade/src/portfolio.rs`:

```rust
use soroban_sdk::{Address, Env, Vec};
use common::Asset;
use crate::storage::{get_oracle_contract, get_user_positions};
use crate::errors::AutoTradeError;

/// Calculate total portfolio value in base currency
pub fn calculate_portfolio_value(env: &Env, user: &Address) -> Result<i128, AutoTradeError> {
    let oracle_id = get_oracle_contract(env);
    let oracle = OracleContractClient::new(env, &oracle_id);
    
    let positions = get_user_positions(env, user);
    let mut total_value = 0i128;
    
    for position in positions.iter() {
        // Convert each position to base currency
        let value_in_base = oracle.convert_to_base(&position.amount, &position.asset)
            .map_err(|_| AutoTradeError::ConversionFailed)?;
        
        total_value = total_value.checked_add(value_in_base)
            .ok_or(AutoTradeError::Overflow)?;
    }
    
    Ok(total_value)
}

/// Get portfolio breakdown by asset
pub fn get_portfolio_breakdown(env: &Env, user: &Address) -> Vec<AssetValue> {
    let oracle_id = get_oracle_contract(env);
    let oracle = OracleContractClient::new(env, &oracle_id);
    
    let positions = get_user_positions(env, user);
    let mut breakdown = Vec::new(env);
    
    for position in positions.iter() {
        let value_in_base = oracle.convert_to_base(&position.amount, &position.asset)
            .unwrap_or(0);
        
        breakdown.push_back(AssetValue {
            asset: position.asset.clone(),
            amount: position.amount,
            value_in_base,
        });
    }
    
    breakdown
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AssetValue {
    pub asset: Asset,
    pub amount: i128,
    pub value_in_base: i128,
}
```

## Step 5: Update Risk Calculations

In `contracts/auto_trade/src/risk.rs`:

```rust
/// Check if trade exceeds position limit (in base currency)
pub fn check_position_limit(
    env: &Env,
    user: &Address,
    new_position_amount: i128,
    new_position_asset: &Asset,
    max_position_pct: u32,
) -> Result<(), AutoTradeError> {
    let oracle_id = get_oracle_contract(env);
    let oracle = OracleContractClient::new(env, &oracle_id);
    
    // Get total portfolio value
    let total_value = calculate_portfolio_value(env, user)?;
    
    // Convert new position to base currency
    let new_position_value = oracle.convert_to_base(&new_position_amount, new_position_asset)
        .map_err(|_| AutoTradeError::ConversionFailed)?;
    
    // Check if new position exceeds limit
    let max_allowed = total_value.checked_mul(max_position_pct as i128)
        .and_then(|v| v.checked_div(100))
        .ok_or(AutoTradeError::Overflow)?;
    
    if new_position_value > max_allowed {
        return Err(AutoTradeError::PositionLimitExceeded);
    }
    
    Ok(())
}
```

## Step 6: Add Fee Calculations

Calculate fees in base currency:

```rust
/// Calculate total fees paid (in base currency)
pub fn calculate_total_fees(env: &Env, user: &Address) -> i128 {
    let oracle_id = get_oracle_contract(env);
    let oracle = OracleContractClient::new(env, &oracle_id);
    
    let trades = get_user_trades(env, user);
    let mut total_fees = 0i128;
    
    for trade in trades.iter() {
        let fee_in_base = oracle.convert_to_base(&trade.fee_amount, &trade.fee_asset)
            .unwrap_or(0);
        total_fees = total_fees.saturating_add(fee_in_base);
    }
    
    total_fees
}
```

## Step 7: Update Contract Interface

Add oracle initialization to `lib.rs`:

```rust
#[contractimpl]
impl AutoTradeContract {
    /// Initialize with oracle contract
    pub fn initialize(env: Env, admin: Address, oracle_contract: Address) {
        storage::set_admin(&env, admin);
        storage::set_oracle_contract(&env, oracle_contract);
    }
    
    /// Get portfolio value in base currency
    pub fn get_portfolio_value(env: Env, user: Address) -> Result<i128, AutoTradeError> {
        portfolio::calculate_portfolio_value(&env, &user)
    }
    
    /// Get portfolio breakdown
    pub fn get_portfolio_breakdown(env: Env, user: Address) -> Vec<AssetValue> {
        portfolio::get_portfolio_breakdown(&env, &user)
    }
}
```

## Usage Example

```rust
// Deploy oracle
let oracle_id = env.register_contract_wasm(None, oracle_wasm);
let oracle = OracleContractClient::new(&env, &oracle_id);

// Initialize oracle with XLM as base
oracle.initialize(&admin, &xlm_asset);

// Set prices
oracle.set_price(&usdc_xlm_pair, &100_000_000); // 1 USDC = 10 XLM
oracle.set_price(&btc_xlm_pair, &500000_0000000); // 1 BTC = 50000 XLM

// Deploy auto_trade
let auto_trade_id = env.register_contract_wasm(None, auto_trade_wasm);
let auto_trade = AutoTradeContractClient::new(&env, &auto_trade_id);

// Initialize with oracle
auto_trade.initialize(&admin, &oracle_id);

// Get portfolio value
let total_value = auto_trade.get_portfolio_value(&user);
// Returns total value in XLM

// Get breakdown
let breakdown = auto_trade.get_portfolio_breakdown(&user);
// Returns: [
//   { asset: USDC, amount: 100, value_in_base: 1000 },
//   { asset: BTC, amount: 0.1, value_in_base: 5000 },
// ]
```

## Performance Considerations

1. **Batch Conversions**: Convert multiple assets in one call when possible
2. **Cache Awareness**: Repeated conversions benefit from 5-minute cache
3. **Error Handling**: Always handle conversion failures gracefully
4. **Base Currency**: Choose liquid base currency (XLM or USDC) for best paths

## Testing Integration

```rust
#[test]
fn test_portfolio_valuation_multi_asset() {
    let env = Env::default();
    
    // Setup oracle
    let oracle_id = env.register_contract(None, OracleContract);
    let oracle = OracleContractClient::new(&env, &oracle_id);
    oracle.initialize(&admin, &xlm);
    
    // Setup prices
    oracle.set_price(&usdc_xlm_pair, &100_000_000);
    oracle.set_price(&btc_xlm_pair, &500000_0000000);
    
    // Setup auto_trade
    let auto_trade_id = env.register_contract(None, AutoTradeContract);
    let client = AutoTradeContractClient::new(&env, &auto_trade_id);
    client.initialize(&admin, &oracle_id);
    
    // Add positions
    add_position(&env, &user, &usdc, 100_0000000); // 100 USDC
    add_position(&env, &user, &btc, 1_0000000);    // 1 BTC
    
    // Calculate total value
    let total = client.get_portfolio_value(&user).unwrap();
    
    // Expected: (100 * 10) + (1 * 50000) = 51000 XLM
    assert_eq!(total, 51000_0000000);
}
```

## Complete Implementation Checklist

- [x] Oracle contract created with conversion system
- [x] Direct conversion implemented
- [x] Path-based conversion with BFS
- [x] Caching system for performance
- [ ] Add oracle dependency to auto_trade
- [ ] Update portfolio.rs with oracle integration
- [ ] Update risk.rs with base currency checks
- [ ] Add oracle contract address to storage
- [ ] Update initialization to accept oracle address
- [ ] Add portfolio value endpoint
- [ ] Add portfolio breakdown endpoint
- [ ] Write integration tests
- [ ] Update deployment scripts

## Next Steps

1. Add oracle dependency to auto_trade/Cargo.toml
2. Implement portfolio valuation functions
3. Update risk management with base currency
4. Add comprehensive integration tests
5. Deploy both contracts and link them
