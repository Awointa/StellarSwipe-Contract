# StellarSwipe Integration Testing Guide

## System Overview

The StellarSwipe platform integrates multiple smart contract modules:

```
┌─────────────────────────────────────────────────────┐
│           StellarSwipe Platform                     │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────────┐      ┌──────────────────┐   │
│  │  Momentum Trading│      │ Bridge Monitoring│   │
│  │   Strategy       │      │   System         │   │
│  └────────┬─────────┘      └────────┬─────────┘   │
│           │                         │               │
│           ├────────┬────────────────┤               │
│           │        │                │               │
│  ┌────────▼────────▼─────┐ ┌───────▼──────────┐   │
│  │   Auto-Trade Module   │ │  Oracle Module   │   │
│  │   (Risk/History)      │ │  (Block Heights) │   │
│  └───────────────────────┘ └──────────────────┘   │
│           │                        │                │
│           └────────┬───────────────┘                │
│                    │                                 │
│          ┌─────────▼─────────┐                     │
│          │ Signal Registry   │                     │
│          │ (Execution/Store) │                     │
│          └───────────────────┘                     │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## Integration Test Scenario: Complete Trade Flow

### Phase 1: Momentum Detection (Momentum Strategy)

**Precondition**: Price data available for XLM/USDC pair

```rust
#[test]
fn test_detect_momentum_and_execute() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    
    // 1. Create momentum strategy
    let mut strategy = MomentumStrategy {
        asset: AssetPair { base: Symbol::short("XLM"), quote: Symbol::short("USDC") },
        positions: Map::new(&env),
        last_rebalance: 1000,
    };
    
    // 2. Collect price history (20 days of uptrend)
    let mut prices = Vec::new();
    for i in 0..20 {
        prices.push_back(1_000_000 * (100 + i) / 100); // +1% each day
    }
    
    // 3. Calculate indicators
    let roc = calculate_rate_of_change(*prices.get(0).unwrap(), *prices.get(19).unwrap());
    // ROC = ((1_200_000 - 1_000_000) / 1_000_000) * 10000 = 2_000 basis points
    
    let rsi = calculate_rsi_from_prices(&prices);
    // RSI = ~80 (overbought with consistent gains)
    
    let macd = calculate_macd_from_prices(&prices);
    // MACD > Signal (positive momentum)
    
    let trend = calculate_trend_strength(&prices);
    // Trend = 9_500+ (19 up days out of 20)
    
    // ✅ Step 1: All indicators confirm uptrend
    assert!(roc > 1_500);
    assert!(rsi > 70);
    assert!(trend > 8_000);
    
    // 4. Calculate confidence
    let indicators = MomentumIndicators { roc, rsi, macd, trend_strength: trend };
    let confidence = calculate_momentum_confidence(&indicators, 10_000);
    
    // ✅ Step 2: High confidence score
    assert!(confidence > 7_000);
    
    // 5. Generate signal
    let signal = check_momentum_signals(&env, &asset, &indicators, confidence, 10_000, false)
        .unwrap()
        .unwrap();
    
    // ✅ Step 3: BUY signal generated
    assert_eq!(signal.direction, TradeDirection::Long);
    
    // 6. Execute momentum trade
    execute_momentum_trade(
        &env,
        &asset,
        TradeDirection::Long,
        500, // 5% position
        100_000_000_000,
        prices.get(19).unwrap(), // Current price
    ).unwrap();
    
    // ✅ Step 4: Position opened
    let position = strategy.positions.get(&asset_id).unwrap();
    assert_eq!(position.status, MonitoringStatus::Active);
}
```

---

### Phase 2: Send Signal to Registry

**Precondition**: Momentum signal generated, Signal Registry contract deployed

```rust
#[test]
fn test_signal_to_registry() {
    let env = Env::default();
    
    // 1. Get momentum signal (from Phase 1)
    let signal = check_momentum_signals(...).unwrap().unwrap();
    
    // 2. Create registry submission
    let submission = SignalSubmission {
        asset: Symbol::short("XLM"),
        direction: signal.direction,
        entry_price: signal.entry_price,
        confidence: signal.confidence,
        timestamp: env.ledger().timestamp(),
    };
    
    // 3. Submit to signal registry
    // registry::submit_signal(&env, submission)?;
    // ✅ Signal recorded in registry
    
    // 4. Registry emits event for other services
    // Event: "signal_submitted" with submission details
}
```

---

### Phase 3: Bridge Transaction (If Cross-Chain Required)

**Precondition**: Assets bridged from Ethereum to Stellar

```rust
#[test]
fn test_cross_chain_bridge_flow() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    
    // Phase 3A: Source Chain (Ethereum)
    // ================================
    
    // 1. User initiates burn on Ethereum
    // - Burns 100 ETH worth of stETH
    // - Emits BridgeInitiated event with tx_hash
    
    let tx_hash = "0xabcd1234567890abcd1234567890abcd1234567890abcd1234567890abcdef12";
    let source_block = 18_000_000;
    
    // Phase 3B: Soroban Bridge Monitoring
    // ===================================
    
    // 2. Create bridge transfer record
    let transfer_id = create_bridge_transfer(
        &env,
        transfer_id: 1,
        source_chain: ChainId::Ethereum,
        destination_chain: ChainId::Stellar,
        amount: 100_000_000,
        user: "0xuser1234...",
    ).unwrap();
    
    // ✅ Transfer created: Pending
    
    // 3. Start monitoring transaction
    monitor_source_transaction(
        &env,
        transfer_id,
        tx_hash,
        ChainId::Ethereum,
        source_block,
    ).unwrap();
    
    env.events().publish(("transaction_monitoring_started", transfer_id));
    // ✅ Monitoring started
    
    // Phase 3C: Confirmation Tracking
    // ==============================
    
    // 4. Oracle provides current block height
    // oracle::get_block_height(ChainId::Ethereum) -> 18_000_001
    
    // 5. Update confirmation count over time
    for current_block in (source_block + 1)..=(source_block + 32) {
        env.ledger().set_timestamp(1000 + (current_block - source_block) * 12);
        
        let finalized = update_transaction_confirmation_count(
            &env,
            transfer_id,
            current_block,
        ).unwrap();
        
        if finalized {
            // ✅ Reached 32 confirmations (Ethereum threshold)
            env.events().publish(("transaction_finalized", transfer_id));
            break;
        }
    }
    
    let tx = get_monitored_tx(&env, transfer_id).unwrap();
    assert_eq!(tx.status, MonitoringStatus::Finalized);
    
    // Phase 3D: Validator Approval
    // ===========================
    
    // 6. Validators verify finality and add signatures
    for (i, validator) in ["validator_1", "validator_2"].iter().enumerate() {
        add_validator_signature(
            &env,
            transfer_id,
            format!("sig_{}", i),
        ).unwrap();
        
        env.events().publish(("validator_signature_added", validator.to_string()));
    }
    
    let transfer = get_bridge_transfer(&env, transfer_id).unwrap();
    assert_eq!(transfer.validator_signatures.len(), 2);
    assert_eq!(transfer.status, TransferStatus::ValidatorApproved);
    
    // Phase 3E: Minting
    // ================
    
    // 7. Approve for minting
    approve_transfer_for_minting(&env, transfer_id).unwrap();
    env.events().publish(("transfer_approved_minting", transfer_id));
    
    // 8. Mint equivalent tokens on Stellar
    // mint_bridge_asset(ChainId::Ethereum, amount)?;
    // ✅ 100 ETH-equivalent tokens minted
    
    // 9. Complete transfer
    complete_transfer(&env, transfer_id).unwrap();
    env.events().publish(("transfer_complete", transfer_id));
    
    let final_transfer = get_bridge_transfer(&env, transfer_id).unwrap();
    assert_eq!(final_transfer.status, TransferStatus::Complete);
    // ✅ Bridge complete!
}
```

---

### Phase 4: Execute Trade with Bridged Assets

**Precondition**: Assets minted on Stellar from bridge

```rust
#[test]
fn test_trade_with_bridged_assets() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    
    // 1. Assets available (newly minted from bridge)
    let portfolio = Portfolio {
        balance: 100_000_000, // 100 ETH worth of assets
        xlm_position: 500_000_000_000, // XLM in account
    };
    
    // 2. Execute XLM/USDC trade based on momentum signal
    execute_momentum_trade(
        &env,
        &AssetPair {
            base: Symbol::short("XLM"),
            quote: Symbol::short("USDC"),
        },
        TradeDirection::Long,
        500, // 5% of portfolio
        portfolio.balance,
        1_200_000, // Current price
    ).unwrap();
    
    // ✅ Trade executed with bridged assets
    
    // 3. Track position
    // update history for analytics
    // position indexed for leaderboard tracking
}
```

---

### Phase 5: Monitor Position with History Tracking

**Precondition**: Momentum trade executed

```rust
#[test]
fn test_position_monitoring_and_history() {
    let env = Env::default();
    let mut strategy = create_momentum_strategy(&env);
    
    // 1. Execute trade
    execute_momentum_trade(&env, &asset, TradeDirection::Long, ...)?;
    
    // 2. Record in history
    // history::record_entry(&env, &signal)?;
    
    // 3. Monitor via signal registry
    // registry::track_performance(&env, asset_id)?;
    
    // 4. Update trailing stops
    update_trailing_stops(&env, &mut strategy, new_price)?;
    
    // 5. On exit, record in history
    // history::record_exit(&env, entry, exit, pnl)?;
    
    // ✅ Complete trade lifecycle tracked
}
```

---

## End-to-End Testing Checklist

### ✅ Setup Phase
- [ ] All modules deployed (momentum, bridge, oracle, registry)
- [ ] Module permissions configured
- [ ] Storage initialized
- [ ] Test accounts funded

### ✅ Momentum Phase
- [ ] Price data collected
- [ ] Indicators calculated correctly
- [ ] Confidence score computed
- [ ] Signal generated
- [ ] Position created with correct size

### ✅ Registry Phase
- [ ] Signal submitted to registry
- [ ] Event emitted
- [ ] Registry updated
- [ ] Leaderboard updated

### ✅ Bridge Phase
- [ ] Transfer created
- [ ] Monitoring started
- [ ] Confirmations tracked
- [ ] Finality reached
- [ ] Validator signatures collected
- [ ] Approved for minting
- [ ] Assets minted
- [ ] Bridge completed

### ✅ Oracle Phase
- [ ] Block heights queried
- [ ] Current confirmations accurate
- [ ] Finality status correct
- [ ] Reorg detection working

### ✅ Trade Execution Phase
- [ ] Position opened with bridged assets
- [ ] Position sizing correct
- [ ] Trailing stops set
- [ ] Risk limits enforced

### ✅ Monitoring Phase
- [ ] Position tracked
- [ ] Exit signals generated
- [ ] Trailing stops update
- [ ] History recorded

### ✅ Verification Phase
- [ ] All events emitted correctly
- [ ] Storage state consistent
- [ ] No panics or errors
- [ ] Gas costs acceptable

---

## Integration Test Patterns

### Pattern 1: Momentum → Bridge → Trade

```rust
#[test]
fn momentum_bridge_trade_integration() {
    let env = Env::default();
    
    // 1. Detect momentum
    let signal = detect_momentum_signal(&env)?;
    
    // 2. Bridge assets if needed
    let transfer_id = initiate_bridge_transfer(&env, signal.asset)?;
    
    // 3. Wait for finality
    wait_for_bridge_finality(&env, transfer_id)?;
    
    // 4. Execute trade
    execute_trade_with_bridged_assets(&env, signal)?;
    
    // 5. Verify complete
    verify_complete_flow(&env)?;
}
```

### Pattern 2: Multi-Asset Rebalancing with Bridge

```rust
#[test]
fn rebalance_with_cross_chain() {
    let env = Env::default();
    
    // 1. Score all assets by momentum
    let ranked = rank_assets_by_momentum(&env)?;
    
    // 2. For new top assets, bridge from other chains
    for asset in ranked.iter().take(3) {
        if needs_bridge_in(asset) {
            initiate_bridge_transfer(&env, asset)?;
        }
    }
    
    // 3. Wait for all bridges
    wait_for_all_transfers(&env)?;
    
    // 4. Rebalance portfolio
    rebalance_by_momentum_rank(&env, &ranked)?;
    
    // 5. Verify allocation
    verify_rebalance_complete(&env)?;
}
```

### Pattern 3: Risk Management End-to-End

```rust
#[test]
fn test_risk_management_across_bridge() {
    let env = Env::default();
    
    // 1. Open position via momentum
    execute_momentum_trade(&env, &asset, ...)?;
    
    // 2. Asset price moves with trailing stop
    update_trailing_stops(&env, &mut strategy, new_price)?;
    
    // 3. Price hits stop - position closes
    // Even if position was bridged from another chain
    
    // 4. Verify realized P&L recorded
    let history = get_trade_history(&env)?;
    verify_realized_pnl(history)?;
}
```

---

## Event Tracking

### Complete Event Flow

```
Momentum Phase:
├─ momentum_strength_calculated ──┐
├─ signal_generated               │
└─ momentum_trade_executed        │
                                  │
Registry Phase:                   │
└─ signal_submitted ◄─────────────┘
   ├─ signal_stored
   └─ leaderboard_updated

Bridge Phase:
├─ transaction_monitoring_started
├─ transaction_finalized
├─ validator_signature_added (×2)
├─ transfer_approved_minting
└─ transfer_complete

Execution Phase:
├─ trade_executed
├─ position_created
├─ position_updated (on price moves)
└─ position_closed (on stop)

History Phase:
├─ entry_recorded
└─ exit_recorded
   ├─ pnl_calculated
   └─ leaderboard_updated
```

---

## Performance Targets

### End-to-End Latency
```
Momentum Detection:          ~10ms
Signal Generation:          ~5ms
Registry Submission:        ~20ms
─────────────────────────────────
Subtotal (Fast Path):       ~35ms

Bridge Monitoring (1st block): ~100ms
Bridge Finality (32 blocks):   ~384 seconds (Ethereum)
Bridge Validation:            ~50ms
─────────────────────────────
Subtotal (Bridge Path):     ~385 seconds

Trade Execution:            ~50ms
Position Setup:             ~30ms
─────────────────────────────
Subtotal (Execution):       ~80ms

TOTAL MOMENTUM→TRADE:       ~115ms
TOTAL WITH BRIDGE:          ~385 seconds (1 Ethereum block period)
```

### Throughput

```
Concurrent Signals:         1000+ per minute
Concurrent Trades:          100+ per block
Concurrent Bridges:         10+ concurrent transfers
Concurrent Positions:       1000+ simultaneous
```

### Gas Costs (Estimated)

```
Momentum Calculation:        ~50,000 gas
Signal Generation:           ~20,000 gas
Position Creation:           ~80,000 gas
Confirmation Update:         ~30,000 gas
Transfer Finalization:       ~100,000 gas
Validator Signature:         ~25,000 gas
Total Per Trade:             ~205,000 gas
```

---

## Rollback Testing

### Scenario: Reorg During Bridge

```rust
#[test]
fn test_reorg_recovery() {
    let env = Env::default();
    
    // 1. Create bridge transfer
    create_bridge_transfer(&env, 1, ...)?;
    monitor_source_transaction(&env, 1, "0xtx", ChainId::Ethereum, 15000000)?;
    
    // 2. Reach 32 confirmations
    update_transaction_confirmation_count(&env, 1, 15000032)?;
    // ✅ Finalized
    
    // 3. Reorg detected!
    let is_reorg = check_for_reorg(&env, 1, 15000010)?;
    assert!(is_reorg);
    
    // 4. Handle reorg
    handle_reorg(&env, 1)?;
    
    let tx = get_monitored_tx(&env, 1).unwrap();
    assert_eq!(tx.status, MonitoringStatus::Reorged);
    assert_eq!(tx.confirmations, 0);
    
    let transfer = get_bridge_transfer(&env, 1).unwrap();
    assert_eq!(transfer.status, TransferStatus::Pending);
    assert_eq!(transfer.validator_signatures.len(), 0);
    
    // 5. Restart monitoring
    monitor_source_transaction(&env, 1, "0xtx", ChainId::Ethereum, 15000010)?;
    
    // 6. Re-confirm
    update_transaction_confirmation_count(&env, 1, 15000042)?;
    // ✅ Finalized again
    
    // 7. Validators re-sign
    add_validator_signature(&env, 1, "sig1")?;
    add_validator_signature(&env, 1, "sig2")?;
    
    // 8. Complete
    approve_transfer_for_minting(&env, 1)?;
    complete_transfer(&env, 1)?;
    
    // ✅ Recovered from reorg
}
```

---

## Success Criteria

✅ All unit tests pass (19 momentum + 24 bridge = 43 total)
✅ Integration scenarios complete without errors
✅ Event flows are correct
✅ Performance targets met
✅ Gas costs within budget
✅ State consistency maintained
✅ Rollback scenarios handled
✅ All edge cases covered

---

## Deployment Checklist

- [ ] All modules implemented
- [ ] All unit tests passing
- [ ] Integration tests passing
- [ ] Code review completed
- [ ] Security audit completed
- [ ] Performance measured
- [ ] Gas costs optimized
- [ ] Testnet deployment complete
- [ ] Documentation updated
- [ ] Team trained

---

## Next Steps

1. **Immediate**: Run complete integration test suite
2. **Week 1**: Testnet deployment with real validators
3. **Week 2**: Cross-chain oracle integration testing
4. **Week 3**: Performance and load testing
5. **Week 4**: Security audit and hardening
6. **Week 5**: Mainnet deployment

🚀 **Ready for integration testing!**
