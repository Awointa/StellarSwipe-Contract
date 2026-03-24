# Oracle Contract - Quick Start Guide

## Installation

The oracle contract is part of the StellarSwipe workspace. No additional installation needed.

## Building

```bash
cd stellar-swipe/contracts/oracle
make build
```

## Testing

```bash
make test
```

## Usage Example

```rust
use soroban_sdk::{Env, Address};

// Initialize contract
let env = Env::default();
let contract_id = env.register_contract(None, OracleContract);
let client = OracleContractClient::new(&env, &contract_id);

let admin = Address::generate(&env);
client.initialize(&admin);

// Register oracles
let oracle1 = Address::generate(&env);
let oracle2 = Address::generate(&env);
let oracle3 = Address::generate(&env);

client.register_oracle(&admin, &oracle1);
client.register_oracle(&admin, &oracle2);
client.register_oracle(&admin, &oracle3);

// Oracles submit prices
client.submit_price(&oracle1, &100_000_000); // $100
client.submit_price(&oracle2, &101_000_000); // $101
client.submit_price(&oracle3, &99_000_000);  // $99

// Calculate consensus (automatically updates reputations)
let consensus_price = client.calculate_consensus();
println!("Consensus price: {}", consensus_price);

// Check oracle reputation
let reputation = client.get_oracle_reputation(&oracle1);
println!("Oracle 1 reputation: {}", reputation.reputation_score);
println!("Oracle 1 weight: {}", reputation.weight);
println!("Oracle 1 accuracy: {}/{}", 
    reputation.accurate_submissions, 
    reputation.total_submissions
);
```

## Key Concepts

### Reputation Score (0-100)
- **90-100**: Excellent - Weight 10
- **75-89**: Good - Weight 5
- **60-74**: Average - Weight 2
- **50-59**: Below Average - Weight 1
- **<50**: Poor - Weight 0 (cannot submit)

### Accuracy Thresholds
- **Accurate**: Within 1% of consensus
- **Moderate**: Within 5% of consensus
- **Inaccurate**: >5% deviation

### Slashing
- **>20% deviation**: -20 reputation points
- **Signature failure**: -30 reputation points

## Monitoring

```rust
// Get all oracles
let oracles = client.get_oracles();

// Check each oracle's reputation
for oracle in oracles.iter() {
    let rep = client.get_oracle_reputation(&oracle);
    println!("Oracle: {:?}", oracle);
    println!("  Reputation: {}", rep.reputation_score);
    println!("  Weight: {}", rep.weight);
    println!("  Accuracy: {}%", 
        (rep.accurate_submissions * 100) / rep.total_submissions.max(1)
    );
}

// Get latest consensus
if let Some(consensus) = client.get_consensus_price() {
    println!("Latest consensus: {}", consensus.price);
    println!("Timestamp: {}", consensus.timestamp);
    println!("Oracles: {}", consensus.num_oracles);
}
```

## Best Practices

1. **Regular Submissions**: Oracles should submit prices regularly to maintain reputation
2. **Accurate Data**: Stay within 1% of consensus for best reputation
3. **Monitor Weight**: Check weight regularly; weight 0 means removal
4. **Recovery**: Improve accuracy to recover reputation after poor performance
5. **Minimum Oracles**: System maintains at least 2 oracles for reliability

## Troubleshooting

### Oracle cannot submit price
- Check if oracle is registered: `get_oracles()`
- Check oracle weight: `get_oracle_reputation(oracle).weight`
- If weight is 0, oracle has been removed due to poor performance

### Reputation not improving
- Ensure submissions are within 5% of consensus
- Check `avg_deviation` - lower is better
- May need 20+ accurate submissions to significantly improve

### All oracles removed
- System maintains minimum 2 oracles
- If all perform poorly, worst performers are kept
- Admin can manually register new oracles

## Events

Monitor these events for oracle activity:
- `price_submitted` - Oracle submitted a price
- `consensus_reached` - Consensus calculated
- `weight_adjusted` - Oracle weight changed
- `oracle_slashed` - Oracle penalized
- `oracle_removed` - Oracle removed from system

## Support

For issues or questions, refer to:
- README.md - Full documentation
- IMPLEMENTATION_SUMMARY.md - Technical details
- test.rs - Usage examples
