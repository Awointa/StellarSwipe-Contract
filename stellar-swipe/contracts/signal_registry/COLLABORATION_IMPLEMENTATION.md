# Collaborative Signals Implementation

## Overview
This implementation adds support for multi-author collaborative signals where multiple providers can co-author signals and share performance credit and fees.

## Files Created/Modified

### New Files
1. **`src/collaboration.rs`** - Core collaboration logic
   - Author management
   - Approval workflow
   - Fee/reward distribution

2. **`src/test_collaboration.rs`** - Unit tests for collaboration features

### Modified Files
1. **`src/types.rs`** - Added `is_collaborative` field to Signal struct
2. **`src/errors.rs`** - Added CollaborationError enum
3. **`src/events.rs`** - Added collaboration events
4. **`src/lib.rs`** - Added collaboration module and public API functions

## Key Features

### 1. Collaborative Signal Creation
```rust
pub fn create_collaborative_signal(
    env: Env,
    primary_author: Address,
    co_authors: Vec<Address>,
    contribution_pcts: Vec<u32>,
    // ... signal parameters
) -> Result<u64, AdminError>
```

- Primary author creates signal with co-authors
- Contribution percentages in basis points (10000 = 100%)
- Signal starts in `Pending` status
- Primary author auto-approves

### 2. Approval Workflow
```rust
pub fn approve_collaborative_signal(
    env: Env,
    signal_id: u64,
    approver: Address,
) -> Result<(), AdminError>
```

- Each co-author must approve
- Signal publishes when all approve
- Status changes from `Pending` to `Active`

### 3. Collaboration Details
```rust
pub fn get_collaboration_details(
    env: Env,
    signal_id: u64,
) -> Option<Vec<Author>>
```

Returns author list with:
- Address
- Contribution percentage
- Approval status

### 4. Fee & Performance Distribution
```rust
pub fn distribute_collaborative_rewards(
    env: &Env,
    authors: &Vec<Author>,
    total_fees: i128,
    total_roi: i128,
) -> Vec<(Address, i128, i128)>
```

Splits fees and ROI proportionally based on contribution percentages.

## Data Structures

### Author
```rust
pub struct Author {
    pub address: Address,
    pub contribution_pct: u32,  // Basis points (10000 = 100%)
    pub has_approved: bool,
}
```

### CollaborationStatus
```rust
pub enum CollaborationStatus {
    PendingApproval,
    Approved,
    Rejected,
}
```

## Events

1. **`collab_signal_created`** - Emitted when collaborative signal is created
2. **`collab_signal_approved`** - Emitted when co-author approves
3. **`collab_signal_published`** - Emitted when all approvals received

## Validation Rules

1. **Contribution Percentages**
   - Must sum to exactly 10000 (100%)
   - Number of percentages must match number of authors (primary + co-authors)

2. **Approval**
   - Only listed co-authors can approve
   - Cannot approve twice
   - All must approve before publication

3. **Authorization**
   - Primary author must authorize creation
   - Each co-author must authorize their approval

## Usage Example

```rust
// Create collaborative signal (60/40 split)
let mut co_authors = Vec::new(&env);
co_authors.push_back(co_author_address);

let mut contribution_pcts = Vec::new(&env);
contribution_pcts.push_back(6000); // Primary: 60%
contribution_pcts.push_back(4000); // Co-author: 40%

let signal_id = client.create_collaborative_signal(
    &primary_author,
    &co_authors,
    &contribution_pcts,
    &String::from_str(&env, "XLM/USDC"),
    &SignalAction::Buy,
    &1000000,
    &String::from_str(&env, "Bullish on XLM"),
    &(env.ledger().timestamp() + 86400),
    &SignalCategory::SwingTrade,
    &Vec::new(&env),
    &RiskLevel::Medium,
);

// Co-author approves
client.approve_collaborative_signal(&signal_id, &co_author_address);

// Signal is now Active and published
```

## Integration Points

### With Existing Systems

1. **Performance Tracking** - When `record_trade_execution` is called, fees and ROI should be distributed among co-authors using `distribute_collaborative_rewards`

2. **Provider Stats** - Each co-author's stats should be updated proportionally based on their contribution percentage

3. **Fee Management** - Platform fees should be calculated on total, then provider fees split among co-authors

## Testing

Run tests:
```bash
cargo test test_collaboration
```

Test coverage:
- ✅ Create collaborative signal
- ✅ Approve collaborative signal
- ✅ Invalid contribution percentages
- ✅ Unauthorized approval attempts
- ✅ Double approval prevention

## Future Enhancements

1. **Rejection Workflow** - Allow co-authors to reject signals
2. **Timeout Mechanism** - Auto-approve or cancel after 48 hours
3. **Edit Collaboration** - Allow primary author to modify co-author list
4. **Contribution Disputes** - Mechanism to resolve percentage disagreements

## Definition of Done

✅ Collaborative signals can be created  
✅ All co-authors must approve before publication  
✅ Performance stats split by contribution  
✅ Fees distributed proportionally  
✅ Unit tests cover collaboration scenarios  
✅ Events emitted for all collaboration actions  

## Notes

- Minimal implementation focusing on core functionality
- Uses basis points (10000 = 100%) for precise percentage calculations
- Primary author auto-approves to simplify workflow
- Signal marked with `is_collaborative` flag for easy identification
