# Collaboration Integration Examples

## Integration with Performance Tracking

### Update `record_trade_execution` in `lib.rs`

Add this code after updating signal stats and before updating provider stats:

```rust
// After line: signals.set(signal_id, signal.clone());

// Handle collaborative signal fee/stat distribution
if signal.is_collaborative {
    if let Some(authors) = collaboration::get_collaborative_signal(&env, signal_id) {
        // Distribute rewards among co-authors
        let distributions = collaboration::distribute_collaborative_rewards(
            &authors,
            breakdown.provider_fee,  // From fee calculation
            signal.total_roi,
        );
        
        // Update each co-author's provider stats
        let mut provider_stats_map = Self::get_provider_stats_map(&env);
        
        for (author_addr, fee_share, roi_share) in distributions.iter() {
            let mut author_stats = provider_stats_map
                .get(author_addr.clone())
                .unwrap_or_default();
            
            // Update stats proportionally
            performance::update_provider_performance(
                &mut author_stats,
                &old_status,
                &new_status,
                roi_share,
                (signal.total_volume * author.contribution_pct as i128) / 10000,
            );
            
            provider_stats_map.set(author_addr, author_stats);
        }
        
        Self::save_provider_stats_map(&env, &provider_stats_map);
    }
} else {
    // Original single-provider logic
    let mut provider_stats_map = Self::get_provider_stats_map(&env);
    // ... existing code ...
}
```

## Integration with Fee Distribution

### Update `collect_and_distribute_fee` in `fees.rs`

Replace the provider fee transfer section:

```rust
// After calculating breakdown

// Distribute provider fees
if is_collaborative {
    // Get collaboration details
    let authors = collaboration::get_collaborative_signal(env, signal_id)?;
    let splits = collaboration::split_provider_fee(&authors, breakdown.provider_fee);
    
    // Transfer to each co-author
    for i in 0..splits.len() {
        let (idx, amount) = splits.get(i).unwrap();
        let author = authors.get(idx).unwrap();
        
        // TODO: Actual token transfer
        // token_client.transfer(
        //     &env.current_contract_address(),
        //     &author.address,
        //     &amount
        // );
        
        // Emit individual fee event
        emit_collaborative_fee_distributed(
            env,
            signal_id,
            author.address,
            amount,
        );
    }
} else {
    // Original single provider transfer
    // token_client.transfer(
    //     &env.current_contract_address(),
    //     &provider,
    //     &breakdown.provider_fee
    // );
}

// Platform fee transfer remains the same
// token_client.transfer(
//     &env.current_contract_address(),
//     &platform_treasury,
//     &breakdown.platform_fee
// );
```

## Add Helper Function to `lib.rs`

Add this helper to check if a signal is collaborative:

```rust
fn is_signal_collaborative(env: &Env, signal_id: u64) -> bool {
    let signals = Self::get_signals_map(env);
    if let Some(signal) = signals.get(signal_id) {
        signal.is_collaborative
    } else {
        false
    }
}
```

## Frontend Integration Example

### Create Collaborative Signal

```typescript
// Frontend TypeScript/JavaScript
async function createCollaborativeSignal() {
  const primaryAuthor = "GXXX..."; // Stellar address
  const coAuthors = [
    "GYYY...",  // Co-author 1
    "GZZZ...",  // Co-author 2
  ];
  
  const contributionPcts = [
    6000,  // Primary: 60%
    2500,  // Co-author 1: 25%
    1500,  // Co-author 2: 15%
  ];
  
  const signalId = await contract.create_collaborative_signal({
    primary_author: primaryAuthor,
    co_authors: coAuthors,
    contribution_pcts: contributionPcts,
    asset_pair: "XLM/USDC",
    action: { tag: "Buy" },
    price: 1000000n,
    rationale: "Bullish on XLM due to...",
    expiry: Date.now() + 86400000,
    category: { tag: "SwingTrade" },
    tags: [],
    risk_level: { tag: "Medium" },
  });
  
  console.log("Collaborative signal created:", signalId);
}
```

### Approve Collaborative Signal

```typescript
async function approveSignal(signalId: number) {
  await contract.approve_collaborative_signal({
    signal_id: signalId,
    approver: currentUserAddress,
  });
  
  console.log("Signal approved");
}
```

### Display Collaboration Details

```typescript
async function getCollaborationInfo(signalId: number) {
  const details = await contract.get_collaboration_details({
    signal_id: signalId,
  });
  
  if (details) {
    console.log("Co-authors:");
    details.forEach(author => {
      console.log(`- ${author.address}: ${author.contribution_pct / 100}%`);
      console.log(`  Approved: ${author.has_approved}`);
    });
  }
}
```

## Event Listening

### Listen for Collaboration Events

```typescript
// Subscribe to collaboration events
contract.on("collab_signal_created", (event) => {
  console.log("New collaborative signal:", event.signal_id);
  console.log("Co-authors:", event.authors);
  // Notify co-authors to approve
});

contract.on("collab_signal_approved", (event) => {
  console.log("Signal approved by:", event.approver);
  // Update UI to show approval status
});

contract.on("collab_signal_published", (event) => {
  console.log("Signal published:", event.signal_id);
  // Move signal to active feed
});
```

## Database Schema (Off-chain Indexer)

### Store Collaboration Data

```sql
-- Collaborative signals table
CREATE TABLE collaborative_signals (
    signal_id BIGINT PRIMARY KEY,
    created_at TIMESTAMP,
    status VARCHAR(20), -- 'pending', 'approved', 'rejected'
    FOREIGN KEY (signal_id) REFERENCES signals(id)
);

-- Co-authors table
CREATE TABLE signal_authors (
    id SERIAL PRIMARY KEY,
    signal_id BIGINT,
    author_address VARCHAR(56),
    contribution_pct INTEGER,
    has_approved BOOLEAN DEFAULT FALSE,
    approved_at TIMESTAMP,
    FOREIGN KEY (signal_id) REFERENCES collaborative_signals(signal_id)
);

-- Index for quick lookups
CREATE INDEX idx_signal_authors_signal ON signal_authors(signal_id);
CREATE INDEX idx_signal_authors_author ON signal_authors(author_address);
```

## API Endpoints (Backend)

### REST API Examples

```typescript
// GET /api/signals/:id/collaboration
app.get('/api/signals/:id/collaboration', async (req, res) => {
  const signalId = req.params.id;
  const details = await contract.get_collaboration_details({ signal_id: signalId });
  
  res.json({
    signal_id: signalId,
    is_collaborative: details !== null,
    authors: details || [],
    all_approved: details?.every(a => a.has_approved) || false,
  });
});

// POST /api/signals/:id/approve
app.post('/api/signals/:id/approve', async (req, res) => {
  const signalId = req.params.id;
  const approver = req.body.approver;
  
  await contract.approve_collaborative_signal({
    signal_id: signalId,
    approver: approver,
  });
  
  res.json({ success: true });
});

// GET /api/users/:address/pending-approvals
app.get('/api/users/:address/pending-approvals', async (req, res) => {
  const address = req.params.address;
  
  // Query database for signals awaiting this user's approval
  const pending = await db.query(`
    SELECT cs.signal_id, s.asset_pair, s.rationale
    FROM collaborative_signals cs
    JOIN signal_authors sa ON cs.signal_id = sa.signal_id
    JOIN signals s ON cs.signal_id = s.id
    WHERE sa.author_address = $1
      AND sa.has_approved = FALSE
      AND cs.status = 'pending'
  `, [address]);
  
  res.json(pending.rows);
});
```

## Testing Integration

### Integration Test Example

```rust
#[test]
fn test_collaborative_signal_full_workflow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let primary = Address::generate(&env);
    let co_author = Address::generate(&env);

    client.initialize(&admin);

    // Create collaborative signal
    let signal_id = client.create_collaborative_signal(
        &primary,
        &vec![&env, co_author.clone()],
        &vec![&env, 7000, 3000],
        // ... other params
    );

    // Verify pending status
    let signal = client.get_signal(&signal_id).unwrap();
    assert_eq!(signal.status, SignalStatus::Pending);
    assert!(signal.is_collaborative);

    // Co-author approves
    client.approve_collaborative_signal(&signal_id, &co_author);

    // Verify active status
    let signal = client.get_signal(&signal_id).unwrap();
    assert_eq!(signal.status, SignalStatus::Active);

    // Record trade execution
    client.record_trade_execution(
        &Address::generate(&env),
        &signal_id,
        1000000,
        1100000,
        10000000,
    );

    // Verify both authors' stats updated
    let primary_stats = client.get_provider_stats(&primary).unwrap();
    let co_author_stats = client.get_provider_stats(&co_author).unwrap();
    
    assert!(primary_stats.total_volume > 0);
    assert!(co_author_stats.total_volume > 0);
}
```

## Notification System

### Notify Co-authors

```typescript
async function notifyCoAuthors(signalId: number, authors: string[]) {
  for (const author of authors) {
    await sendNotification({
      to: author,
      type: "collaboration_request",
      data: {
        signal_id: signalId,
        message: "You've been added as a co-author. Please review and approve.",
        action_url: `/signals/${signalId}/approve`,
      },
    });
  }
}
```

---

These integration examples show exactly how to connect the collaboration feature with:
- ✅ Performance tracking system
- ✅ Fee distribution system
- ✅ Frontend UI
- ✅ Backend API
- ✅ Database indexing
- ✅ Event handling
- ✅ Notifications
