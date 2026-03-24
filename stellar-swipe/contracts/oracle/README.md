 feature/signal-categorization-tagging
 feature/signal-categorization-tagging
=======
 feature/oracle-price-conversion
 main
# Oracle Contract - Price Conversion System

## Overview

The Oracle contract provides a price conversion system that translates any asset value to a base currency using direct or path-based conversion. This enables portfolio aggregation across multiple assets.

## Features

✅ **Configurable Base Currency** - Default: XLM, can be changed to USDC or any asset
✅ **Direct Conversion** - Asset → Base currency in one hop
✅ **Path-Based Conversion** - Asset → Intermediate(s) → Base (up to 3 hops)
✅ **Automatic Path Finding** - BFS algorithm finds shortest conversion path
✅ **Conversion Rate Caching** - 5-minute cache for performance
✅ **Overflow Protection** - Safe arithmetic operations

## Architecture

 feature/signal-categorization-tagging
```
=======
 main
contracts/oracle/
├── src/
│   ├── lib.rs          # Contract interface
│   ├── conversion.rs   # Core conversion logic
│   ├── storage.rs      # Data persistence
│   └── errors.rs       # Error types
├── Cargo.toml
└── Makefile
 feature/signal-categorization-tagging
```
=======

 main

## Key Functions

### Public Interface

```rust
// Initialize with base currency
fn initialize(env: Env, admin: Address, base_currency: Asset)

// Set price for asset pair
fn set_price(env: Env, pair: AssetPair, price: i128) -> Result<(), OracleError>

// Get price for asset pair
fn get_price(env: Env, pair: AssetPair) -> Result<i128, OracleError>

// Convert amount to base currency
fn convert_to_base(env: Env, amount: i128, asset: Asset) -> Result<i128, OracleError>

// Get/Set base currency
fn get_base_currency(env: Env) -> Asset
fn set_base_currency(env: Env, asset: Asset)
```

### Conversion Logic

**Direct Conversion:**
```rust
// If XLM/USDC pair exists with price P:
// amount_in_base = amount * P / PRECISION
```

**Path-Based Conversion:**
```rust
// For TOKEN → USDC → XLM:
// 1. TOKEN → USDC using TOKEN/USDC price
// 2. USDC → XLM using USDC/XLM price
// Result: amount in XLM
```

## Usage Examples

### 1. Initialize Oracle

```rust
let xlm = Asset {
    code: String::from_str(&env, "XLM"),
    issuer: None,
};
client.initialize(&admin, &xlm);
```

### 2. Set Prices

```rust
// 1 USDC = 10 XLM
let pair = AssetPair {
    base: usdc.clone(),
    quote: xlm.clone(),
};
client.set_price(&pair, &100_000_000); // 10 * 10^7
```

### 3. Convert to Base Currency

```rust
// Convert 100 USDC to XLM
let result = client.convert_to_base(&100_0000000, &usdc).unwrap();
// Result: 1000 XLM (100 * 10)
```

### 4. Change Base Currency

```rust
// Switch from XLM to USDC
client.set_base_currency(&usdc);
```

## Performance

| Operation | Target | Implementation |
|-----------|--------|----------------|
| Direct conversion | <100ms | ✅ Single storage read + arithmetic |
| Path conversion (2 hops) | <300ms | ✅ BFS + 2 conversions |
| Path finding | <500ms | ✅ BFS with max 3 hops |
| Cache hit | <10ms | ✅ Temporary storage lookup |

## Caching Strategy

- **Conversion rates** cached for 5 minutes (60 ledgers)
- **Price data** persists for 24 hours (17,280 ledgers)
- **Available pairs** stored persistently
- Cache invalidation on base currency change

## Error Handling

```rust
pub enum OracleError {
    PriceNotFound = 1,        // No price data for pair
    NoConversionPath = 2,     // No path from asset to base
    InvalidPath = 3,          // Path construction failed
    ConversionOverflow = 4,   // Arithmetic overflow
    Unauthorized = 5,         // Permission denied
    InvalidAsset = 6,         // Invalid asset format
    StalePrice = 7,           // Price data too old
}
```

## Edge Cases Handled

✅ **Same asset conversion** - Returns amount unchanged
✅ **No conversion path** - Returns NoConversionPath error
✅ **Circular paths** - Prevented by visited tracking in BFS
✅ **Overflow protection** - checked_mul/checked_div throughout
✅ **Base currency change** - Cache invalidated automatically

## Testing

Run tests:
```bash
cd stellar-swipe
cargo test --package oracle
```

Test coverage:
- ✅ Initialize and get base currency
- ✅ Set and get price
- ✅ Direct conversion (USDC → XLM)
- ✅ Same asset conversion (XLM → XLM)
- ✅ Base currency change
- ✅ Path-based conversion (multi-hop)
- ✅ Cache functionality
- ✅ Error scenarios
=======
# Oracle Reputation & Automatic Weight Adjustment

This Soroban smart contract implements an oracle reputation system that tracks oracle accuracy and automatically adjusts weights to favor better-performing oracles.

## Features

### 1. Oracle Reputation Tracking
- Tracks total submissions and accurate submissions per oracle
- Calculates average deviation from consensus
- Maintains reputation score (0-100)
- Records last slash timestamp for consistency scoring

### 2. Reputation Calculation
Reputation is calculated using three components:
- **60% Accuracy Rate**: Based on submissions within acceptable deviation
- **30% Deviation Score**: Lower average deviation = higher score
- **10% Consistency Score**: No slashes in the past 7 days = bonus points

### 3. Automatic Weight Adjustment
Weights are automatically adjusted based on reputation:
- **90-100**: Weight 10 (High reputation)
- **75-89**: Weight 5 (Good reputation)
- **60-74**: Weight 2 (Average reputation)
- **50-59**: Weight 1 (Below average)
- **<50**: Weight 0 (Removed)

### 4. Accuracy Tracking
After consensus is established, each oracle's submission is evaluated:
- **Accurate**: Within 1% of consensus
- **Moderately Accurate**: Within 5% of consensus
- **Inaccurate**: >5% deviation

### 5. Slashing Mechanism
Oracles are penalized for:
- **Major Deviation (>20%)**: -20 reputation points
- **Signature Verification Failure**: -30 reputation points

### 6. Oracle Removal
Oracles are removed if:
- Reputation score falls below 50
- Accuracy rate <50% over 100+ submissions
- System maintains minimum of 2 oracles

## Contract Functions

### Admin Functions
- `initialize(admin: Address)` - Initialize contract
- `register_oracle(admin: Address, oracle: Address)` - Register new oracle
- `remove_oracle(admin: Address, oracle: Address)` - Manually remove oracle

### Oracle Functions
- `submit_price(oracle: Address, price: i128)` - Submit price data
- `calculate_consensus()` - Calculate consensus and update reputations

### Query Functions
- `get_oracle_reputation(oracle: Address)` - Get oracle stats
- `get_oracles()` - Get all registered oracles
- `get_consensus_price()` - Get latest consensus price

## Data Structures

### OracleReputation
```rust
pub struct OracleReputation {
    pub total_submissions: u32,
    pub accurate_submissions: u32,
    pub avg_deviation: i128,
    pub reputation_score: u32,
    pub weight: u32,
    pub last_slash: u64,
}
```

### ConsensusPriceData
```rust
pub struct ConsensusPriceData {
    pub price: i128,
    pub timestamp: u64,
    pub num_oracles: u32,
}
```

## Events

The contract emits the following events:
- `oracle_removed` - When an oracle is removed
- `weight_adjusted` - When oracle weight changes
- `oracle_slashed` - When an oracle is penalized
- `price_submitted` - When a price is submitted
- `consensus_reached` - When consensus is calculated
 main

## Building

```bash
 feature/signal-categorization-tagging
 feature/signal-categorization-tagging
=======
 feature/oracle-price-conversion
 main
cd contracts/oracle
make build
```

Output: `target/wasm32-unknown-unknown/release/oracle.wasm`

## Deployment

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/oracle.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY
```

## Integration with Portfolio

The oracle can be integrated with the auto_trade contract for portfolio valuation:

```rust
// In portfolio.rs
use oracle::convert_to_base;

pub fn get_total_value(env: &Env, user: Address) -> i128 {
    let positions = get_positions(env, &user);
    let mut total = 0i128;
    
    for position in positions.iter() {
        let value = convert_to_base(env, position.amount, position.asset)?;
        total = total.checked_add(value).unwrap();
    }
    
    total
}
```

## Future Enhancements

- [ ] Oracle price feeds (Band Protocol integration)
- [ ] Volume-weighted path selection
- [ ] Multi-path arbitrage detection
- [ ] Price staleness checks
- [ ] Admin authorization
- [ ] Event emission for conversions

## Definition of Done

✅ Base currency configurable and stored
✅ Direct conversion working for all pairs
✅ Path-based conversion with BFS
✅ Conversion rate caching implemented
✅ Unit tests cover various scenarios
✅ Performance requirements met
✅ Error handling comprehensive
✅ Documentation complete
=======
make build
```

## Testing

```bash
make test
```

## Usage Example

```rust
// Initialize contract
client.initialize(&admin);

// Register oracles
client.register_oracle(&admin, &oracle1);
client.register_oracle(&admin, &oracle2);
client.register_oracle(&admin, &oracle3);

// Oracles submit prices
client.submit_price(&oracle1, &100_000_000);
client.submit_price(&oracle2, &101_000_000);
client.submit_price(&oracle3, &99_000_000);

// Calculate consensus (automatically updates reputations)
let consensus = client.calculate_consensus();

// Check oracle reputation
let reputation = client.get_oracle_reputation(&oracle1);
println!("Reputation: {}", reputation.reputation_score);
println!("Weight: {}", reputation.weight);
```

## Edge Cases Handled

1. **New Oracle**: Starts with default weight of 1 and reputation of 50
2. **Reputation Recovery**: Oracles can improve reputation through accurate submissions
3. **Minimum Oracles**: System maintains at least 2 oracles even if all perform poorly
4. **Oracle Manipulation**: Sudden reputation drops are detected via slashing mechanism

## Validation Tests

All validation scenarios from the requirements are covered:
- ✅ Submit prices from 3 oracles (1 accurate, 1 moderate, 1 poor)
- ✅ Run reputation calculation, verify scores
- ✅ Verify weights adjusted correctly
- ✅ Submit consistently bad data, verify oracle removal
- ✅ Test reputation recovery after improvement

## Definition of Done

- ✅ Oracle accuracy tracked per submission
- ✅ Reputation calculated from accuracy + deviation
- ✅ Weights adjusted automatically based on reputation
- ✅ Slashing implemented for poor performance
- ✅ Unit tests verify reputation logic
- ✅ Events emitted on weight changes
 main
