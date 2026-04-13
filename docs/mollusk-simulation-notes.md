# Mollusk Simulation Notes

## Fastest Way To Estimate CU For A Transaction

Use `process_transaction_instructions` and read
`result.compute_units_consumed`.

```rust
use mollusk_svm::Mollusk;

let mut mollusk = Mollusk::default();
// Optional if the simulated transaction is large.
// mollusk.compute_budget.compute_unit_limit = 1_400_000;

let result = mollusk.process_transaction_instructions(&instructions, &accounts);

println!("CU: {}", result.compute_units_consumed);
println!("Program result: {:?}", result.program_result);
```

Notes:

- This is simulated CU from actual execution, not a static estimate.
- If the transaction fails partway through, `compute_units_consumed` only
  covers the instructions that actually executed.
- This is the fastest direct transaction-level path in Mollusk because it
  executes all instructions in one shared message/context and returns
  `compute_units_consumed` directly.
- For a single instruction, the short form is:

```rust
let result = mollusk.process_instruction(&instruction, &accounts);
println!("CU: {}", result.compute_units_consumed);
```

If you want it as fast as possible in practice:

- Reuse one `Mollusk` instance across runs.
- Use `process_transaction_instructions`, not `process_and_validate_*`.
- Avoid `process_instruction_chain` unless you specifically want standalone
  per-instruction execution with persisted state between calls.
- Keep the `accounts` slice minimal.
- Only use `MolluskContext` if account-store convenience matters more than raw
  speed.

## Instruction Meta Privilege Overrides

This was added as an opt-in privilege override, not a hard runtime disable.

The public API lives on `Mollusk` as
`instruction_account_privilege_overrides`. The compiler path uses it while
building the `Message`, so default behavior stays unchanged unless you opt in.

Usage:

```rust
use mollusk_svm::Mollusk;

let mut mollusk = Mollusk::default();
mollusk.instruction_account_privilege_overrides.force_signer = true;
mollusk.instruction_account_privilege_overrides.force_writable = true;
```

Meaning:

- `force_signer = true`: every instruction meta is compiled as signer
- `force_writable = true`: every instruction meta is compiled as writable

Important:

- This changes the compiled transaction message, not the original
  `Instruction`.
- This is intentionally non-standard and should only be used for targeted
  simulation scenarios.

Coverage:

- `harness/tests/system_program.rs`
- `harness/tests/transaction_instructions.rs`

Verification commands:

```bash
cargo test -p mollusk-svm --test system_program
cargo test -p mollusk-svm --test transaction_instructions test_missing_signer_fails
cargo test -p mollusk-svm --test transaction_instructions test_missing_signers_can_be_forced_for_transaction
cargo test -p mollusk-svm --no-run
```

Known caveat:

Running the full `transaction_instructions` suite without SBF artifacts can
still fail because one existing test expects
`target/deploy/test_program_primary.so`. That is pre-existing and unrelated to
the privilege override change.
