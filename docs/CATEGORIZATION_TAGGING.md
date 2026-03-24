# Signal Categorization and Tagging System

## Overview

This implementation adds a comprehensive tagging and categorization system to the StellarSwipe signal registry, enabling users to filter and discover signals based on strategy, asset type, and risk level.

## Implementation Summary

### Files Created/Modified

1. **Created: `categories.rs`**
   - `SignalCategory` enum (SwingTrade, DayTrade, LongTerm, Scalping, Breakout, Reversal, Momentum)
   - `RiskLevel` enum (Low, Medium, High)
   - Tag validation and deduplication functions
   - Tag popularity tracking
   - Auto-suggestion based on rationale keywords

2. **Modified: `types.rs`**
   - Added `category`, `tags`, and `risk_level` fields to `Signal` struct

3. **Modified: `lib.rs`**
   - Updated `create_signal()` to accept category, tags, and risk_level
   - Added `add_tags_to_signal()` for adding tags to existing signals
   - Added `get_signals_filtered()` for advanced filtering
   - Added `get_popular_tags()` to retrieve trending tags
   - Added `suggest_tags()` for auto-suggestion based on rationale

4. **Modified: `events.rs`**
   - Added `emit_tags_added()` event

5. **Created: `test_categories.rs`**
   - Comprehensive unit tests for all tagging functionality

## Key Features

### 1. Signal Categories
Predefined categories for signal classification:
- **SwingTrade**: 1-7 days holding period
- **DayTrade**: <24 hours
- **LongTerm**: >7 days
- **Scalping**: <1 hour
- **Breakout**: Technical breakout patterns
- **Reversal**: Trend reversal signals
- **Momentum**: Momentum-based plays

### 2. Risk Levels
Three-tier risk classification:
- **Low**: Conservative signals
- **Medium**: Moderate risk/reward
- **High**: Aggressive, high-risk signals

### 3. Custom Tags
- Free-form strings (max 20 characters)
- Alphanumeric + hyphen/underscore only
- Max 10 tags per signal
- Automatic deduplication
- Popularity tracking

### 4. Advanced Filtering
Filter signals by:
- One or more categories
- One or more tags (any match)
- One or more risk levels
- Pagination support (offset + limit)

### 5. Tag Discovery
- **Popular Tags**: Get most-used tags across all signals
- **Auto-Suggest**: Keyword-based tag suggestions from rationale text
  - Keywords: breakout, bullish, bearish, oversold, overbought, reversal, momentum, high-risk, earnings

## API Reference

### Create Signal with Categorization
```rust
pub fn create_signal(
    env: Env,
    provider: Address,
    asset_pair: String,
    action: SignalAction,
    price: i128,
    rationale: String,
    expiry: u64,
    category: SignalCategory,
    tags: Vec<String>,
    risk_level: RiskLevel,
) -> Result<u64, AdminError>
```

### Add Tags to Existing Signal
```rust
pub fn add_tags_to_signal(
    env: Env,
    provider: Address,
    signal_id: u64,
    tags: Vec<String>,
) -> Result<(), AdminError>
```

### Filter Signals
```rust
pub fn get_signals_filtered(
    env: Env,
    categories: Option<Vec<SignalCategory>>,
    tags: Option<Vec<String>>,
    risk_levels: Option<Vec<RiskLevel>>,
    offset: u32,
    limit: u32,
) -> Vec<Signal>
```

### Get Popular Tags
```rust
pub fn get_popular_tags(
    env: Env,
    limit: u32,
) -> Vec<(String, u32)>
```

### Suggest Tags
```rust
pub fn suggest_tags(
    env: Env,
    rationale: String,
) -> Vec<String>
```

## Usage Examples

### Submit Signal with Category and Tags
```rust
let mut tags = Vec::new(&env);
tags.push_back(String::from_str(&env, "bullish"));
tags.push_back(String::from_str(&env, "breakout"));

let signal_id = client.create_signal(
    &provider,
    &String::from_str(&env, "XLM/USDC"),
    &SignalAction::Buy,
    &1_000_000,
    &String::from_str(&env, "Strong breakout above resistance"),
    &(env.ledger().timestamp() + 86400),
    &SignalCategory::SwingTrade,
    &tags,
    &RiskLevel::Medium,
);
```

### Filter by Category
```rust
let mut categories = Vec::new(&env);
categories.push_back(SignalCategory::SwingTrade);

let signals = client.get_signals_filtered(
    &Some(categories),
    &None,
    &None,
    &0,
    &10,
);
```

### Filter by Tags
```rust
let mut tags = Vec::new(&env);
tags.push_back(String::from_str(&env, "bullish"));

let signals = client.get_signals_filtered(
    &None,
    &Some(tags),
    &None,
    &0,
    &10,
);
```

### Combined Filtering
```rust
let mut categories = Vec::new(&env);
categories.push_back(SignalCategory::SwingTrade);

let mut tags = Vec::new(&env);
tags.push_back(String::from_str(&env, "momentum"));

let mut risk_levels = Vec::new(&env);
risk_levels.push_back(RiskLevel::Medium);

let signals = client.get_signals_filtered(
    &Some(categories),
    &Some(tags),
    &Some(risk_levels),
    &0,
    &10,
);
```

## Validation Rules

### Tag Validation
- Length: 1-20 characters
- Characters: a-z, A-Z, 0-9, hyphen (-), underscore (_)
- Max tags per signal: 10
- Automatic deduplication on submission

### Category Validation
- Must be one of the predefined `SignalCategory` enum values

### Risk Level Validation
- Must be one of: Low, Medium, High

## Edge Cases Handled

1. **Duplicate Tags**: Automatically deduplicated
2. **Max Tag Limit**: Enforced at 10 tags per signal
3. **Invalid Characters**: Rejected with `InvalidParameter` error
4. **Tag Length**: Enforced 1-20 character limit
5. **Unauthorized Tag Addition**: Only signal provider can add tags
6. **Multiple Category Matches**: Signal can only have one category but can be filtered by multiple

## Testing

Run the test suite:
```bash
cargo test test_categories
```

### Test Coverage
- ✅ Create signal with category and tags
- ✅ Add tags to existing signal
- ✅ Max tag limit enforcement
- ✅ Tag deduplication
- ✅ Filter by category
- ✅ Filter by tags
- ✅ Filter by risk level
- ✅ Combined filters
- ✅ Popular tags tracking
- ✅ Tag auto-suggestion
- ✅ Pagination

## Events Emitted

### TagsAdded
```rust
topics: ("tags_added", signal_id, provider)
data: tag_count
```

## Storage

### Tag Popularity
Stored in instance storage:
```rust
Map<String, u32> // tag -> usage_count
```

## Performance Considerations

- Filtering is O(n) where n = total active signals
- Tag matching uses byte comparison for efficiency
- Popularity tracking increments on signal creation and tag addition
- Deduplication is O(n*m) where n = tags, m = unique tags (small numbers)

## Future Enhancements

1. Tag moderation/blacklist
2. Multi-language tag support
3. Tag synonyms/aliases
4. Category-specific tag suggestions
5. Machine learning-based tag recommendations
6. Tag trending over time windows

## Definition of Done

✅ Signals can be categorized with predefined categories
✅ Custom tags supported (max 10 per signal)
✅ Filtering by category, tags, and risk level works
✅ Tag popularity tracked and retrievable
✅ Unit tests cover all tagging scenarios
✅ Tag validation enforces constraints
✅ Auto-suggestion based on rationale keywords
✅ Events emitted for tag operations
✅ Pagination support for filtered results
✅ Deduplication prevents duplicate tags
