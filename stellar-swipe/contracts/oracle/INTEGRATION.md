# Oracle Integration Example

This document shows how the Oracle Reputation contract integrates with the existing StellarSwipe contracts.

## Architecture Overview

```
┌─────────────────────┐
│  Signal Registry    │
│   (Trading Signals) │
└──────────┬──────────┘
           │
           │ Uses price data
           ▼
┌─────────────────────┐
│  Oracle Contract    │◄──── Oracle 1 (Weight: 10)
│  (Price Feeds)      │◄──── Oracle 2 (Weight: 5)
│                     │◄──── Oracle 3 (Weight: 2)
└──────────┬──────────┘
           │
           │ Provides reliable prices
           ▼
┌─────────────────────┐
│   Auto Trade        │
│  (Execute Trades)   │
└─────────────────────┘
```

## Integration Points

### 1. Price Feed for Trading Signals

The Signal Registry can use oracle prices to validate signal prices:

```rust
// In signal_registry contract
pub fn validate_signal_price(
    env: Env,
    oracle_contract: Address,
    asset_pair: String,
    signal_price: i128,
) -> Result<bool, Error> {
    // Get consensus price from oracle
    let oracle_client = OracleContractClient::new(&env, &oracle_contract);
    let consensus = oracle_client.get_consensus_price()
        .ok_or(Error::NoPriceData)?;
    
    // Check if signal price is within reasonable range (e.g., 10%)
    let deviation = ((signal_price - consensus.price).abs() * 10000) / consensus.price;
    
    if deviation > 1000 { // 10%
        return Ok(false); // Signal price too far from market
    }
    
    Ok(true)
}
```

### 2. Auto Trade Execution

The Auto Trade contract can use oracle prices for execution:

```rust
// In auto_trade contract
pub fn execute_trade_with_oracle(
    env: Env,
    oracle_contract: Address,
    signal_id: u64,
    amount: i128,
) -> Result<(), Error> {
    // Get current market price from oracle
    let oracle_client = OracleContractClient::new(&env, &oracle_contract);
    let consensus = oracle_client.get_consensus_price()
        .ok_or(Error::NoPriceData)?;
    
    // Execute trade at oracle price
    execute_trade_internal(&env, signal_id, amount, consensus.price)?;
    
    Ok(())
}
```

### 3. Risk Management

Use oracle reputation to assess data quality:

```rust
pub fn check_price_reliability(
    env: Env,
    oracle_contract: Address,
) -> Result<bool, Error> {
    let oracle_client = OracleContractClient::new(&env, &oracle_contract);
    let oracles = oracle_client.get_oracles();
    
    let mut high_quality_oracles = 0;
    let mut total_weight = 0;
    
    for oracle in oracles.iter() {
        let rep = oracle_client.get_oracle_reputation(&oracle);
        if rep.reputation_score >= 75 {
            high_quality_oracles += 1;
        }
        total_weight += rep.weight;
    }
    
    // Require at least 2 high-quality oracles and total weight >= 10
    Ok(high_quality_oracles >= 2 && total_weight >= 10)
}
```

## Complete Integration Example

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct TradingSystem;

#[contractimpl]
impl TradingSystem {
    /// Execute a trade using oracle price validation
    pub fn execute_validated_trade(
        env: Env,
        oracle_contract: Address,
        signal_registry: Address,
        trader: Address,
        signal_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        trader.require_auth();
        
        // 1. Get signal from registry
        let signal_client = SignalRegistryClient::new(&env, &signal_registry);
        let signal = signal_client.get_signal(signal_id)
            .ok_or(Error::SignalNotFound)?;
        
        // 2. Get oracle consensus price
        let oracle_client = OracleContractClient::new(&env, &oracle_contract);
        let consensus = oracle_client.get_consensus_price()
            .ok_or(Error::NoPriceData)?;
        
        // 3. Validate signal price against oracle
        let deviation = ((signal.price - consensus.price).abs() * 10000) 
            / consensus.price;
        
        if deviation > 1000 { // 10% threshold
            return Err(Error::PriceDeviationTooHigh);
        }
        
        // 4. Check oracle reliability
        let oracles = oracle_client.get_oracles();
        let mut total_weight = 0;
        for oracle in oracles.iter() {
            let rep = oracle_client.get_oracle_reputation(&oracle);
            total_weight += rep.weight;
        }
        
        if total_weight < 10 {
            return Err(Error::InsufficientOracleQuality);
        }
        
        // 5. Execute trade at oracle price
        execute_trade(&env, trader, signal_id, amount, consensus.price)?;
        
        Ok(())
    }
}
```

## Deployment Sequence

1. **Deploy Oracle Contract**
   ```bash
   soroban contract deploy \
     --wasm target/wasm32-unknown-unknown/release/oracle.wasm \
     --network testnet
   ```

2. **Initialize Oracle Contract**
   ```bash
   soroban contract invoke \
     --id <ORACLE_CONTRACT_ID> \
     --network testnet \
     -- initialize \
     --admin <ADMIN_ADDRESS>
   ```

3. **Register Oracles**
   ```bash
   soroban contract invoke \
     --id <ORACLE_CONTRACT_ID> \
     --network testnet \
     -- register_oracle \
     --admin <ADMIN_ADDRESS> \
     --oracle <ORACLE_1_ADDRESS>
   ```

4. **Update Existing Contracts**
   - Add oracle_contract_id to Signal Registry storage
   - Add oracle_contract_id to Auto Trade storage
   - Update trade execution logic to use oracle prices

## Monitoring Dashboard

Track oracle health in real-time:

```rust
pub fn get_oracle_health_report(
    env: Env,
    oracle_contract: Address,
) -> OracleHealthReport {
    let client = OracleContractClient::new(&env, &oracle_contract);
    let oracles = client.get_oracles();
    
    let mut report = OracleHealthReport {
        total_oracles: oracles.len(),
        active_oracles: 0,
        total_weight: 0,
        avg_reputation: 0,
        consensus_age: 0,
    };
    
    let mut total_reputation = 0;
    
    for oracle in oracles.iter() {
        let rep = client.get_oracle_reputation(&oracle);
        if rep.weight > 0 {
            report.active_oracles += 1;
        }
        report.total_weight += rep.weight;
        total_reputation += rep.reputation_score;
    }
    
    if report.total_oracles > 0 {
        report.avg_reputation = total_reputation / report.total_oracles;
    }
    
    if let Some(consensus) = client.get_consensus_price() {
        report.consensus_age = env.ledger().timestamp() - consensus.timestamp;
    }
    
    report
}
```

## Benefits of Integration

1. **Reliable Price Data**: Weighted consensus from multiple sources
2. **Automatic Quality Control**: Poor oracles automatically downweighted
3. **Fraud Prevention**: Slashing mechanism deters manipulation
4. **Self-Healing**: System maintains quality through reputation tracking
5. **Transparency**: All oracle performance metrics on-chain

## Next Steps

1. Deploy oracle contract to testnet
2. Register initial oracle providers
3. Update Signal Registry to validate prices
4. Update Auto Trade to use oracle prices
5. Monitor oracle performance
6. Adjust thresholds based on real-world data

## Security Considerations

- **Oracle Selection**: Carefully vet initial oracle providers
- **Admin Keys**: Secure admin private keys for oracle management
- **Threshold Tuning**: Adjust accuracy thresholds based on asset volatility
- **Monitoring**: Set up alerts for low oracle quality
- **Backup Plan**: Have manual override capability for emergencies
