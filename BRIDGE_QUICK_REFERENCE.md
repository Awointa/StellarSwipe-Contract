# Bridge Monitoring - Quick Reference Guide

## Chain Configurations

### Ethereum
```rust
ChainFinalityConfig {
    chain_id: ChainId::Ethereum,
    required_confirmations: 32,
    average_block_time: 12,  // seconds
    reorg_depth_limit: 64,
    verification_method: VerificationMethod::EpochFinality,
}
// Finality time: ~6.4 minutes
```

### Bitcoin
```rust
ChainFinalityConfig {
    chain_id: ChainId::Bitcoin,
    required_confirmations: 6,
    average_block_time: 600,  // seconds
    reorg_depth_limit: 10,
    verification_method: VerificationMethod::Probabilistic,
}
// Finality time: ~60 minutes (6 blocks * 2 for probabilistic)
```

### Polygon
```rust
ChainFinalityConfig {
    chain_id: ChainId::Polygon,
    required_confirmations: 128,
    average_block_time: 2,  // seconds
    reorg_depth_limit: 256,
    verification_method: VerificationMethod::BlockConfirmations,
}
// Finality time: ~4.3 minutes
```

### Binance Smart Chain
```rust
ChainFinalityConfig {
    chain_id: ChainId::BNB,
    required_confirmations: 15,
    average_block_time: 3,  // seconds
    reorg_depth_limit: 20,
    verification_method: VerificationMethod::BlockConfirmations,
}
// Finality time: ~45 seconds
```

---

## API Quick Reference

### Create Transfer
```rust
create_bridge_transfer(
    env,
    transfer_id,          // u64: unique ID
    ChainId::Ethereum,    // source chain
    ChainId::Polygon,     // destination chain
    1000000,              // i128: amount
    "user_address",       // String: recipient
)?
```

### Start Monitoring
```rust
monitor_source_transaction(
    env,
    transfer_id,
    "0xabcd1234...",      // tx_hash
    ChainId::Ethereum,
    15000000,             // block_number
)?
```

### Update Confirmations
```rust
let is_finalized = update_transaction_confirmation_count(
    env,
    transfer_id,
    15000032,             // current_block on source chain
)?;
```

### Check for Reorg
```rust
let is_reorg = check_for_reorg(
    env,
    transfer_id,
    15000010,             // current_block (may be lower if reorg)
)?;

if is_reorg {
    handle_reorg(env, transfer_id)?;
}
```

### Add Validator Signature
```rust
add_validator_signature(
    env,
    transfer_id,
    "validator_sig_1",    // String: signature
)?;
// Requires 2 signatures to approve
```

### Approve for Minting
```rust
approve_transfer_for_minting(env, transfer_id)?;
// Requires ValidatorApproved status
```

### Complete Transfer
```rust
complete_transfer(env, transfer_id)?;
```

---

## Status Transitions

### Monitoring States
```
Pending
  ↓
Confirming (confirmations accumulating)
  ↓
Finalized (enough confirmations)
  OR
  ↘
Reorged (chain reorganization detected)
  ↓
Pending (restart monitoring)
  OR
  ↘
Failed (monitoring timeout)
```

### Transfer States
```
Pending (waiting for source finality)
  ↓
Finalized (source confirmed)
  ↓
ValidatorApproved (2+ signatures)
  ↓
Minting (in progress)
  ↓
Complete (success)
OR
↘ Failed (any step failed)
```

---

## Monitoring Loop Example

```rust
// 1. Create transfer
create_bridge_transfer(env, 1, ChainId::Ethereum, ChainId::Polygon, 1e6, "user")?;

// 2. User submits Ethereum transaction
monitor_source_transaction(env, 1, "0xabcd...", ChainId::Ethereum, 15000000)?;

// 3. Oracle periodically updates confirmations
loop {
    let current_block = query_ethereum_height();
    
    // Check for reorg
    if check_for_reorg(env, 1, current_block)? {
        handle_reorg(env, 1)?;
        monitor_source_transaction(env, 1, "0xabcd...", ChainId::Ethereum, current_block)?;
        continue;
    }
    
    // Update confirmations
    let finalized = update_transaction_confirmation_count(env, 1, current_block)?;
    
    if finalized {
        break;  // Proceed to validator signatures
    }
    
    std::thread::sleep(Duration::from_secs(30));
}

// 4. Validators add signatures
add_validator_signature(env, 1, "sig_validator1")?;
add_validator_signature(env, 1, "sig_validator2")?;

// 5. Approve for minting
approve_transfer_for_minting(env, 1)?;

// 6. Mint on destination chain (external process)
// ...

// 7. Mark complete
complete_transfer(env, 1)?;
```

---

## Finality Time Estimates

| Chain | Confirmations | Block Time | Total Time |
|-------|---------------|-----------|-----------|
| Ethereum | 32 | 12s | ~6.4 min |
| Polygon | 128 | 2s | ~4.3 min |
| BSC | 15 | 3s | ~45 sec |
| Bitcoin | 6 (×2) | 10 min | ~120 min |

---

## Events for Monitoring

```rust
// Subscribe to these in external systems:

"transaction_monitoring_started"
  -> (transfer_id, chain_id, tx_hash)

"transaction_finalized"
  -> (transfer_id, confirmations)

"reorg_detected"
  -> (transfer_id, old_block, new_block)

"reorg_handled"
  -> (transfer_id, confirmations)

"monitoring_failed"
  -> (transfer_id, timestamp)

"bridge_transfer_created"
  -> (transfer_id, source_chain, dest_chain)

"validator_signature_added"
  -> (transfer_id, signature_count)

"transfer_approved_minting"
  -> (transfer_id, timestamp)

"transfer_complete"
  -> (transfer_id, timestamp)
```

---

## Error Handling

```rust
// Common error types

String::from_linear(env, "Transaction not found")
String::from_linear(env, "Invalid amount")
String::from_linear(env, "Transfer not found")
String::from_linear(env, "Signature already added")
String::from_linear(env, "Transfer not approved by validators")
```

---

## Storage Access Patterns

```rust
// Get monitored transaction
let tx = get_monitored_tx(env, transfer_id);

// Get bridge transfer
let transfer = get_bridge_transfer(env, transfer_id);

// Get chain config
let config = get_chain_finality_config(env, ChainId::Ethereum)?;

// Set custom config
set_chain_finality_config(env, &custom_config);
```

---

## Testing Examples

```rust
#[test]
fn test_ethereum_finality() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    
    // Monitor at block 100
    monitor_source_transaction(env, 1, "0xabcd", ChainId::Ethereum, 100).unwrap();
    
    // Update at 132 (32 confirmations)
    let finalized = update_transaction_confirmation_count(env, 1, 132).unwrap();
    assert!(finalized);
}

#[test]
fn test_bitcoin_probabilistic() {
    let env = Env::default();
    
    monitor_source_transaction(env, 2, "0xefgh", ChainId::Bitcoin, 700000).unwrap();
    
    // Bitcoin requires 6 * 2 = 12 confirmations
    let finalized = update_transaction_confirmation_count(env, 2, 700012).unwrap();
    assert!(finalized);
}

#[test]
fn test_reorg_handling() {
    let env = Env::default();
    
    monitor_source_transaction(env, 3, "0xijkl", ChainId::Polygon, 5000).unwrap();
    
    // Reorg detected if current_block < block_number
    let is_reorg = check_for_reorg(env, 3, 4990).unwrap();
    assert!(is_reorg);
    
    handle_reorg(env, 3).unwrap();
    
    let tx = get_monitored_tx(env, 3).unwrap();
    assert_eq!(tx.status, MonitoringStatus::Reorged);
}
```

---

## Performance Tips

1. **Batch Updates**: Call `update_transaction_confirmation_count()` for multiple transfers at once
2. **Limit Queries**: Use `get_pending_monitored_transactions(limit)` with reasonable limits
3. **Cache Configs**: Chain finality configs rarely change, cache locally
4. **Event Filtering**: Subscribe to specific events in monitoring systems

---

## Common Scenarios

### Scenario 1: Fast Finality (Polygon)
- User initiates: 0 sec
- Monitoring starts: 0 sec
- Confirmations tracked: 0-256 sec (128 blocks × 2s)
- **Finality: ~4 minutes**

### Scenario 2: Slow Finality (Bitcoin)
- User initiates: 0 sec
- Monitoring starts: 0 sec
- Confirmations tracked: 0-7200 sec (12 blocks × 600s)
- **Finality: ~2 hours**

### Scenario 3: Reorg Scenario
- Transaction at block 1000: FINALIZED
- Reorg detected: 995 < 1000
- State reset: confirmations=0, status=Pending
- Re-monitor from new tip
- **Delay: +5 minutes for new finality**

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Not finalized | Confirmations insufficient | Wait longer or verify block height |
| Reorg keeps happening | Chain too young | Wait beyond reorg depth |
| Timeout | Never saw on source | Verify tx_hash, check source chain |
| Signature error | Duplicate added | Check existing signatures |
| Minting failed | Status not ValidatorApproved | Need 2 signatures |

---

## Constants

```rust
const ETHEREUM_FINALITY: u32 = 32;      // blocks
const POLYGON_FINALITY: u32 = 128;      // blocks
const BSC_FINALITY: u32 = 15;          // blocks
const BITCOIN_FINALITY: u32 = 6;       // blocks (2x for probabilistic)

const ETHEREUM_BLOCK_TIME: u64 = 12;   // seconds
const POLYGON_BLOCK_TIME: u64 = 2;     // seconds
const BSC_BLOCK_TIME: u64 = 3;         // seconds
const BITCOIN_BLOCK_TIME: u64 = 600;   // seconds

const MONITORING_TIMEOUT: u64 = 3600;  // seconds (1 hour)
```

---

## Integration Checklist

- [ ] Add bridge module to main contract
- [ ] Integrate with oracle for block heights
- [ ] Set up event listeners in monitoring service
- [ ] Configure validator signing keys
- [ ] Test on Testnet
- [ ] Deploy to production
- [ ] Monitor event streams
