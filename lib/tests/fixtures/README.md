# QMDL Test Fixtures

This directory contains QMDL capture files used for integration testing of the analysis pipeline.

## Current Fixtures

| File | Description | Expected Results |
|------|-------------|------------------|
| `clean_baseline.qmdl` | Normal cellular activity, no attacks | No warnings (false positive check) |

## Adding New Fixtures

1. Place `.qmdl` files in this directory
2. Keep files small (a few KB is ideal) - trim to just the relevant messages
3. Update the table above with description and expected results
4. Add corresponding test in `lib/tests/qmdl_analysis_tests.rs`

## Future: Programmatic Fixture Generation

For attack scenario fixtures, we can programmatically construct QMDL files:

### Approach

1. **Use existing test vectors** - The byte arrays in `lib/tests/test_lte_parsing.rs`
   provide templates for valid DIAG messages

2. **Modify trigger fields** - Each heuristic looks for specific conditions:
   - **Null Cipher**: Set `cipheringAlgorithm` to `eea0` in SecurityModeCommand
   - **IMSI Request**: Add IdentityRequest with `identityType = imsi` before auth
   - **2G Downgrade**: Set `connectionReleaseRedirection` with 2G ARFCN

3. **Write with QmdlWriter** - Use the existing `QmdlWriter` to create valid QMDL:
   ```rust
   use rayhunter::qmdl::QmdlWriter;
   use rayhunter::diag::MessagesContainer;

   let file = File::create("attack_scenario.qmdl").await?;
   let mut writer = QmdlWriter::new(file);
   writer.write_container(&crafted_container).await?;
   ```

### Heuristic Reference

See `lib/src/analysis/` for each analyzer's trigger conditions:
- `null_cipher.rs` - Checks for EEA0/EIA0 in SecurityModeCommand
- `imsi_requested.rs` - Detects IMSI requests without prior auth
- `connection_redirect_downgrade.rs` - Detects forced 2G redirections
- `priority_2g_downgrade.rs` - Detects SIB6/7 with 2G priority
- `nas_null_cipher.rs` - Detects null ciphering at NAS layer

## Running Tests

```bash
# Run all tests including fixture-based tests
NO_FIRMWARE_BIN=true cargo test --package rayhunter

# Run only fixture tests
NO_FIRMWARE_BIN=true cargo test --package rayhunter qmdl_analysis
```
