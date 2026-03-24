# TWAP Quick Reference Guide

## Basic Usage

### Store Price Updates
```rust
use crate::history::store_price;

// Automatically stores at 5-minute bucket intervals
store_price(&env, &pair, 10_000_000);
```

### Query Historical Price
```rust
use crate::history::get_historical_price;

// Get price from 1 hour ago
let timestamp = env.ledger().timestamp() - 3600;
let price = get_historical_price(&env, &pair, timestamp);
```

### Calculate TWAP
```rust
use crate::history::calculate_twap;

// 1 hour TWAP
let twap_1h = calculate_twap(&env, &pair, 3600)?;

// 24 hour TWAP
let twap_24h = calculate_twap(&env, &pair, 86400)?;

// 7 day TWAP
let twap_7d = calculate_twap(&env, &pair, 604800)?;
```

### Detect Price Manipulation
```rust
use crate::history::get_twap_deviation;

let current_price = 11_000_000;
let deviation = get_twap_deviation(&env, &pair, current_price, 3600)?;

if deviation > 1000 { // >10%
    // Price manipulation detected
    return Err(OracleError::UnreliablePrice);
}
```

## Contract API Examples

### Via Contract Client
```rust
let client = OracleContractClient::new(&env, &contract_id);

// Get historical price
let historical = client.get_historical_price(&pair, &timestamp);

// Get TWAP windows
let twap_1h = client.get_twap_1h(&pair)?;
let twap_24h = client.get_twap_24h(&pair)?;
let twap_7d = client.get_twap_7d(&pair)?;

// Check deviation
let deviation = client.get_price_deviation(&pair, &current_price, &3600)?;
```

## Common Patterns

### Stop-Loss with TWAP
```rust
let twap_24h = calculate_twap(&env, &pair, 86400)?;
let stop_loss_price = twap_24h * 90 / 100; // 10% below TWAP

if current_price < stop_loss_price {
    execute_stop_loss(&env, &position)?;
}
```

### Performance Tracking
```rust
let entry_price = get_historical_price(&env, &pair, entry_time)?;
let exit_twap = calculate_twap(&env, &pair, 3600)?;
let roi = ((exit_twap - entry_price) * 100) / entry_price;
```

### Flash Crash Protection
```rust
let twap_1h = calculate_twap(&env, &pair, 3600)?;
let deviation = get_twap_deviation(&env, &pair, spot_price, 3600)?;

// Use TWAP if spot price deviates >5%
let safe_price = if deviation > 500 { twap_1h } else { spot_price };
```

## Error Handling

```rust
match calculate_twap(&env, &pair, 86400) {
    Ok(twap) => {
        // Use TWAP
    },
    Err(OracleError::InsufficientHistoricalData) => {
        // Not enough data, use fallback
    },
    Err(e) => {
        // Handle other errors
    }
}
```

## Constants

```rust
const BUCKET_SIZE: u64 = 300;        // 5 minutes
const MAX_BUCKETS: u64 = 2016;       // 7 days
const HOUR: u64 = 3600;              // 1 hour
const DAY: u64 = 86400;              // 24 hours
const WEEK: u64 = 604800;            // 7 days
```

## Storage Keys

```rust
// Key format: (AssetPair, bucket_id)
// bucket_id = timestamp / 300
let bucket = timestamp / BUCKET_SIZE;
let key = (pair.clone(), bucket);
```

## Best Practices

1. **Always check for sufficient data** before calculating TWAP
2. **Use TWAP for stop-loss** instead of spot price
3. **Monitor deviation** to detect manipulation
4. **Store prices regularly** (every 5 minutes recommended)
5. **Handle missing data** gracefully in your logic

## Performance Tips

- TWAP calculation is O(n) where n = window / bucket_size
- 1h TWAP: 12 iterations
- 24h TWAP: 288 iterations
- 7d TWAP: 2016 iterations
- All complete in <500ms

## Data Retention

- **Active**: Last 7 days at 5-minute resolution
- **Pruned**: Automatically after 7 days
- **Future**: Queries for future timestamps return None
