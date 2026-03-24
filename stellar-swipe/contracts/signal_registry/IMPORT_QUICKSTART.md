# Signal Import - Quick Reference

## CSV Format Template

```csv
asset_pair,action,price,rationale,expiry_hours
XLM/USDC,BUY,120000,Technical breakout,24
BTC/USDC,SELL,45000000,Overbought RSI,48
```

## Import CSV

```rust
use soroban_sdk::Bytes;

let csv_data = Bytes::from_slice(&env, b"asset_pair,action,price,rationale,expiry_hours\nXLM/USDC,BUY,120000,Breakout,24");
let result = client.import_signals_csv(&provider, &csv_data, &false);
```

## Validate Only (Dry Run)

```rust
let result = client.import_signals_csv(&provider, &csv_data, &true);
if result.error_count > 0 {
    // Fix errors before actual import
}
```

## Field Requirements

| Field | Type | Range | Example |
|-------|------|-------|---------|
| asset_pair | String | Must contain '/' | XLM/USDC |
| action | String | BUY or SELL | BUY |
| price | Integer | > 0 | 120000 |
| rationale | String | 1-500 chars | Technical breakout |
| expiry_hours | Integer | 1-720 | 24 |

## Error Handling

```rust
let result = client.import_signals_csv(&provider, &csv_data, &false);

println!("Success: {}", result.success_count);
println!("Errors: {}", result.error_count);

// Partial import: valid signals are imported even if some fail
```

## Batch Limits

- Maximum: 100 signals per import
- Exceeding limit returns `BatchSizeExceeded` error
- Split large imports into multiple batches

## External ID Mapping

```csv
asset_pair,action,price,rationale,expiry_hours,external_id
XLM/USDC,BUY,120000,Signal,24,TWITTER_123
```

```rust
let external_id = String::from_str(&env, "TWITTER_123");
let signal_id = client.get_signal_by_external_id(&provider, &external_id);
```

## Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| InvalidFormat | Missing fields | Ensure 5 fields per line |
| InvalidAssetPair | No '/' in pair | Use format: ASSET1/ASSET2 |
| InvalidPrice | Negative/zero | Use positive integers only |
| InvalidAction | Not BUY/SELL | Use BUY or SELL |
| BatchSizeExceeded | > 100 signals | Split into smaller batches |

## Best Practices

1. **Validate First**: Use `validate_only=true` before importing
2. **Batch Size**: Keep batches under 100 signals
3. **Error Handling**: Check `error_count` and handle partial imports
4. **External IDs**: Use unique IDs for tracking
5. **Price Format**: Use smallest unit (stroops for XLM)

## Integration Example

```rust
// Read CSV from external source
let csv_content = fetch_signals_from_twitter();

// Convert to Bytes
let csv_data = Bytes::from_slice(&env, csv_content.as_bytes());

// Validate
let validation = client.import_signals_csv(&provider, &csv_data, &true);
if validation.error_count > 0 {
    log!("Found {} errors in import data", validation.error_count);
    return;
}

// Import
let result = client.import_signals_csv(&provider, &csv_data, &false);
log!("Imported {} signals", result.success_count);
```

## Troubleshooting

### No signals imported (success_count = 0)
- Check CSV format has header row
- Verify all required fields present
- Ensure data is not empty

### High error_count
- Validate CSV format
- Check price values are positive
- Verify action is BUY or SELL
- Ensure asset pairs contain '/'

### BatchSizeExceeded
- Count lines in CSV (excluding header)
- Split into multiple files if > 100
- Import in separate transactions

## Performance Tips

1. **Batch Wisely**: 50-100 signals per batch is optimal
2. **Validate Once**: Don't validate repeatedly
3. **Reuse Data**: Cache CSV data if importing multiple times
4. **Monitor Gas**: Large batches consume more gas

## Security Notes

- Provider authentication required
- All inputs validated
- No code execution from CSV
- Safe parsing with error handling
- Batch limits prevent DoS
