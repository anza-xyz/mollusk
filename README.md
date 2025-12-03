# Mollusk

Mollusk is a lightweight test harness for Solana programs. It provides a
simple interface for testing Solana program executions in a minified
Solana Virtual Machine (SVM) environment.

It does not create any semblance of a validator runtime, but instead
provisions a program execution pipeline directly from lower-level SVM
components.

In summary, the main processor - `process_instruction` - creates minified
instances of Agave's program cache, transaction context, and invoke
context. It uses these components to directly execute the provided
program's ELF using the BPF Loader.

Because it does not use AccountsDB, Bank, or any other large Agave
components, the harness is exceptionally fast. However, it does require
the user to provide an explicit list of accounts to use, since it has
nowhere to load them from.

The test environment can be further configured by adjusting the compute
budget, feature set, or sysvars. These configurations are stored directly
on the test harness (the `Mollusk` struct), but can be manipulated through
a handful of helpers.

* [Single Instructions](#single-instructions)
* [Instruction Chains](#instruction-chains)
* [Stateful Testing with MolluskContext](#stateful-testing-with-molluskcontext)
* [Benchmarking Compute Units](#benchmarking-compute-units)
* [Fixtures](#fixtures)
  * [Generating Fixtures from Mollusk Tests](#generating-fixtures-from-mollusk-tests)
  * [Loading and Executing Fixtures](#loading-and-executing-fixtures)
* [Register tracing](#register-tracing)

## Single Instructions

Both `process_instruction` and `process_and_validate_instruction` deal with
single instructions. The former simply processes the instruction and
returns the result, while the latter processes the instruction and then
performs a series of checks on the result. In both cases, the result is
also returned.

```rust
use {
    mollusk_svm::Mollusk,
    solana_sdk::{account::Account, instruction::{AccountMeta, Instruction}, pubkey::Pubkey},
};

let program_id = Pubkey::new_unique();
let key1 = Pubkey::new_unique();
let key2 = Pubkey::new_unique();

let instruction = Instruction::new_with_bytes(
    program_id,
    &[],
    vec![
        AccountMeta::new(key1, false),
        AccountMeta::new_readonly(key2, false),
    ],
);

let accounts = vec![
    (key1, Account::default()),
    (key2, Account::default()),
];

let mollusk = Mollusk::new(&program_id, "my_program");

// Execute the instruction and get the result.
let result = mollusk.process_instruction(&instruction, &accounts);
```

To apply checks via `process_and_validate_instruction`, developers can use
the `Check` enum, which provides a set of common checks.

```rust
use {
    mollusk_svm::{Mollusk, result::Check},
    solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey
        system_instruction,
        system_program,
    },
};

let sender = Pubkey::new_unique();
let recipient = Pubkey::new_unique();

let base_lamports = 100_000_000u64;
let transfer_amount = 42_000u64;

let instruction = system_instruction::transfer(&sender, &recipient, transfer_amount);
let accounts = [
    (
        sender,
        Account::new(base_lamports, 0, &system_program::id()),
    ),
    (
        recipient,
        Account::new(base_lamports, 0, &system_program::id()),
    ),
];
let checks = vec![
    Check::success(),
    Check::compute_units(system_processor::DEFAULT_COMPUTE_UNITS),
    Check::account(&sender)
        .lamports(base_lamports - transfer_amount)
        .build(),
    Check::account(&recipient)
        .lamports(base_lamports + transfer_amount)
        .build(),
];

Mollusk::default().process_and_validate_instruction(
    &instruction,
    &accounts,
    &checks,
);
```

Note: `Mollusk::default()` will create a new `Mollusk` instance without
adding any provided BPF programs. It will still contain a subset of the
default builtin programs. For more builtin programs, you can add them
yourself or use the `all-builtins` feature.

## Instruction Chains

Both `process_instruction_chain` and
`process_and_validate_instruction_chain` deal with chains of instructions.
The former processes each instruction in the chain and returns the final
result, while the latter processes each instruction in the chain and then
performs a series of checks on each result. In both cases, the final result
is also returned.

```rust
use {
    mollusk_svm::Mollusk,
    solana_sdk::{account::Account, pubkey::Pubkey, system_instruction},
};

let mollusk = Mollusk::default();

let alice = Pubkey::new_unique();
let bob = Pubkey::new_unique();
let carol = Pubkey::new_unique();
let dave = Pubkey::new_unique();

let starting_lamports = 500_000_000;

let alice_to_bob = 100_000_000;
let bob_to_carol = 50_000_000;
let bob_to_dave = 50_000_000;

mollusk.process_instruction_chain(
    &[
        system_instruction::transfer(&alice, &bob, alice_to_bob),
        system_instruction::transfer(&bob, &carol, bob_to_carol),
        system_instruction::transfer(&bob, &dave, bob_to_dave),
    ],
    &[
        (alice, system_account_with_lamports(starting_lamports)),
        (bob, system_account_with_lamports(starting_lamports)),
        (carol, system_account_with_lamports(starting_lamports)),
        (dave, system_account_with_lamports(starting_lamports)),
    ],
);
```

Just like with `process_and_validate_instruction`, developers can use the
`Check` enum to apply checks via `process_and_validate_instruction_chain`.
Notice that `process_and_validate_instruction_chain` takes a slice of
tuples, where each tuple contains an instruction and a slice of checks.
This allows the developer to apply specific checks to each instruction in
the chain. The result returned by the method is the final result of the
last instruction in the chain.

```rust
use {
    mollusk_svm::{Mollusk, result::Check},
    solana_sdk::{account::Account, pubkey::Pubkey, system_instruction},
};

let mollusk = Mollusk::default();

let alice = Pubkey::new_unique();
let bob = Pubkey::new_unique();
let carol = Pubkey::new_unique();
let dave = Pubkey::new_unique();

let starting_lamports = 500_000_000;

let alice_to_bob = 100_000_000;
let bob_to_carol = 50_000_000;
let bob_to_dave = 50_000_000;

mollusk.process_and_validate_instruction_chain(
    &[
        (
            // 0: Alice to Bob
            &system_instruction::transfer(&alice, &bob, alice_to_bob),
            &[
                Check::success(),
                Check::account(&alice)
                    .lamports(starting_lamports - alice_to_bob) // Alice pays
                    .build(),
                Check::account(&bob)
                    .lamports(starting_lamports + alice_to_bob) // Bob receives
                    .build(),
                Check::account(&carol)
                    .lamports(starting_lamports) // Unchanged
                    .build(),
                Check::account(&dave)
                    .lamports(starting_lamports) // Unchanged
                    .build(),
            ],
        ),
        (
            // 1: Bob to Carol
            &system_instruction::transfer(&bob, &carol, bob_to_carol),
            &[
                Check::success(),
                Check::account(&alice)
                    .lamports(starting_lamports - alice_to_bob) // Unchanged
                    .build(),
                Check::account(&bob)
                    .lamports(starting_lamports + alice_to_bob - bob_to_carol) // Bob pays
                    .build(),
                Check::account(&carol)
                    .lamports(starting_lamports + bob_to_carol) // Carol receives
                    .build(),
                Check::account(&dave)
                    .lamports(starting_lamports) // Unchanged
                    .build(),
            ],
        ),
        (
            // 2: Bob to Dave
            &system_instruction::transfer(&bob, &dave, bob_to_dave),
            &[
                Check::success(),
                Check::account(&alice)
                    .lamports(starting_lamports - alice_to_bob) // Unchanged
                    .build(),
                Check::account(&bob)
                    .lamports(starting_lamports + alice_to_bob - bob_to_carol - bob_to_dave) // Bob pays
                    .build(),
                Check::account(&carol)
                    .lamports(starting_lamports + bob_to_carol) // Unchanged
                    .build(),
                Check::account(&dave)
                    .lamports(starting_lamports + bob_to_dave) // Dave receives
                    .build(),
            ],
        ),
    ],
    &[
        (alice, system_account_with_lamports(starting_lamports)),
        (bob, system_account_with_lamports(starting_lamports)),
        (carol, system_account_with_lamports(starting_lamports)),
        (dave, system_account_with_lamports(starting_lamports)),
    ],
);
```

It's important to understand that instruction chains _should not_ be
considered equivalent to Solana transactions. Mollusk does not impose
constraints on instruction chains, such as loaded account keys or size.
Developers should recognize that instruction chains are primarily used for
testing program execution.

## Stateful Testing with MolluskContext

For complex testing scenarios involving multiple instructions or persistent
state between calls, `MolluskContext` provides a stateful wrapper around
`Mollusk`. It automatically manages an account store and provides the same
API methods without requiring explicit account management.

`MolluskContext` is ideal for:
* Testing instruction chains where account state persists between calls
* Complex program interactions that require maintaining account state
* Scenarios where manually managing accounts becomes cumbersome

To use `MolluskContext`, you need to provide an implementation of the
`AccountStore` trait:

```rust
use {
    mollusk_svm::{Mollusk, account_store::AccountStore},
    solana_account::Account,
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_system_interface::instruction as system_instruction,
    std::collections::HashMap,
};

// Simple in-memory account store implementation
#[derive(Default)]
struct InMemoryAccountStore {
    accounts: HashMap<Pubkey, Account>,
}

impl AccountStore for InMemoryAccountStore {
    fn get_account(&self, pubkey: &Pubkey) -> Option<Account> {
        self.accounts.get(pubkey).cloned()
    }

    fn store_account(&mut self, pubkey: Pubkey, account: Account) {
        self.accounts.insert(pubkey, account);
    }
}

let mollusk = Mollusk::default();
let context = mollusk.with_context(InMemoryAccountStore::default());

let alice = Pubkey::new_unique();
let bob = Pubkey::new_unique();

// Execute instructions without managing accounts manually
let instruction1 = system_instruction::transfer(&alice, &bob, 1_000_000);
let result1 = context.process_instruction(&instruction1);

let instruction2 = system_instruction::transfer(&bob, &alice, 500_000);
let result2 = context.process_instruction(&instruction2);

// Account state is automatically preserved between calls
```

The `MolluskContext` API provides the same core methods as `Mollusk`:

* `process_instruction`: Process an instruction with automatic account management
* `process_instruction_chain`: Process a chain of instructions
* `process_and_validate_instruction`: Process and validate an instruction
* `process_and_validate_instruction_chain`: Process and validate an instruction chain

All methods return `InstructionResult`, just like the base `Mollusk` methods.
The `resulting_accounts` field will reflect the final state after execution,
and these accounts are automatically stored in the context's account store.

Note that `HashMap<Pubkey, Account>` implements `AccountStore` directly,
so you can use it as a simple in-memory account store without needing
to implement your own.

## Benchmarking Compute Units
The Mollusk Compute Unit Bencher can be used to benchmark the compute unit
usage of Solana programs. It provides a simple API for developers to write
benchmarks for their programs, which can be checked while making changes to
the program.

A markdown file is generated, which captures all of the compute unit
benchmarks. If a benchmark has a previous value, the delta is also
recorded. This can be useful for developers to check the implications of
changes to the program on compute unit usage.

```rust
use {
    mollusk_svm_bencher::MolluskComputeUnitBencher,
    mollusk_svm::Mollusk,
    /* ... */
};

// Optionally disable logging.
solana_logger::setup_with("");

/* Instruction & accounts setup ... */

let mollusk = Mollusk::new(&program_id, "my_program");

MolluskComputeUnitBencher::new(mollusk)
    .bench(("bench0", &instruction0, &accounts0))
    .bench(("bench1", &instruction1, &accounts1))
    .bench(("bench2", &instruction2, &accounts2))
    .bench(("bench3", &instruction3, &accounts3))
    .must_pass(true)
    .out_dir("../target/benches")
    .execute();

```

The `must_pass` argument can be provided to trigger a panic if any defined
benchmark tests do not pass. `out_dir` specifies the directory where the
markdown file will be written.

Developers can invoke this benchmark test with `cargo bench`. They may need
to add a bench to the project's `Cargo.toml`.

```toml
[[bench]]
name = "compute_units"
harness = false
```

The markdown file will contain entries according to the defined benchmarks.

```markdown
| Name   | CUs   | Delta  |
|--------|-------|--------|
| bench0 | 450   | --     |
| bench1 | 579   | -129   |
| bench2 | 1,204 | +754   |
| bench3 | 2,811 | +2,361 |
```

## Fixtures

Mollusk also supports working with multiple kinds of fixtures, which can
help expand testing capabilities. Note this is all gated behind either the
`fuzz` or `fuzz-fd` feature flags.

A fixture is a structured representation of a test case, containing the
input data, the expected output data, and any additional context required
to run the test. One fixture maps to one instruction.

A classic use case for such fixtures is the act of testing two versions of
a program against each other, to ensure the new version behaves as
expected. The original version's test suite can be used to generate a set
of fixtures, which can then be used as inputs to test the new version.
Although you could also simply replace the program ELF file in the test
suite to achieve a similar result, fixtures provide exhaustive coverage.

### Generating Fixtures from Mollusk Tests

Mollusk is capable of generating fixtures from any defined test case. If
the `EJECT_FUZZ_FIXTURES` environment variable is set during a test run,
Mollusk will serialize every invocation of `process_instruction` into a
fixture, using the provided inputs, current Mollusk configurations, and
result returned. `EJECT_FUZZ_FIXTURES_JSON` can also be set to write the
fixtures in JSON format.

```
EJECT_FUZZ_FIXTURES="./fuzz-fixtures" cargo test-sbf ...
```

Note that Mollusk currently supports two types of fixtures: Mollusk's own
fixture layout and the fixture layout used by the Firedancer team. Both of
these layouts stem from Protobuf definitions.

These layouts live in separate crates, but a snippet of the Mollusk input
data for a fixture can be found below:

```rust
/// Instruction context fixture.
pub struct Context {
    /// The compute budget to use for the simulation.
    pub compute_budget: ComputeBudget,
    /// The feature set to use for the simulation.
    pub feature_set: FeatureSet,
    /// The runtime sysvars to use for the simulation.
    pub sysvars: Sysvars,
    /// The program ID of the program being invoked.
    pub program_id: Pubkey,
    /// Accounts to pass to the instruction.
    pub instruction_accounts: Vec<AccountMeta>,
    /// The instruction data.
    pub instruction_data: Vec<u8>,
    /// Input accounts with state.
    pub accounts: Vec<(Pubkey, Account)>,
}
```

### Loading and Executing Fixtures

Mollusk can also execute fixtures, just like it can with instructions. The
`process_fixture` method will process a fixture and return the result, while
`process_and_validate_fixture` will process a fixture and compare the result
against the fixture's effects.

An additional method, `process_and_partially_validate_fixture`, allows
developers to compare the result against the fixture's effects using a
specific subset of checks, rather than the entire set of effects. This
may be useful if you wish to ignore certain effects, such as compute units
consumed.

```rust
use {
    mollusk_svm::{Mollusk, fuzz::check::FixtureCheck},
    solana_sdk::{account::Account, pubkey::Pubkey, system_instruction},
    std::{fs, path::Path},
};

let mollusk = Mollusk::default();

for file in fs::read_dir(Path::new("fixtures-dir"))? {
    let fixture = Fixture::load_from_blob_file(&entry?.file_name());

    // Execute the fixture and apply partial checks.
    mollusk.process_and_partially_validate_fixture(
       &fixture,
       &[
           FixtureCheck::ProgramResult,
           FixtureCheck::ReturnData,
           FixtureCheck::all_resulting_accounts(),
        ],
    );
}
```

Fixtures can be loaded from files or decoded from raw blobs. These
capabilities are provided by the respective fixture crates.

## Register tracing

Mollusk can be instantiated with the capability to provide register tracing
data from processed instructions. This functionality is gated behind the
`register-tracing` feature flag, which in turn relies on the
`invocation-inspect-callback` flag. To enable it, users can either
construct Mollusk with the `Mollusk::new_debuggable` initializer - allowing
register tracing to be configured directly - or simply set the `SBF_TRACE_DIR`
environment variable, which Mollusk interprets as a signal to turn tracing on
upon instantiation. The latter allows users to take advantage of the
functionality without actually doing any changes to their code.

A default post-instruction callback is provided for storing the
register tracing data in files. It persists the register sets,
the SBPF instructions, and a SHA-256 hash identifying the executable that
was used to generate the tracing data. The motivation behind providing the
SHA-256 identifier is that files may grow in number, and consumers need a
deterministic way to evaluate which shared object should be used when
analyzing the tracing data.

Once enabled register tracing can't be changed afterwards because in nature
it's baked into the program executables at load time. Yet a user may want a
more fine-grained control over when register tracing data should be
collected - for example, only for a specific instruction. Such control could
be achieved by resetting the invocation callback to
`EmptyInvocationInspectCallback` and later by restoring it to
`DefaultRegisterTracingCallback`.

```rust
use mollusk_svm::{register_tracing, EmptyInvocationInspectCallback, Mollusk};

assert!(std::env::var("SBF_TRACE_DIR").is_ok() == true);

/* Mollusk setup .. */
/* Load programs .. */

/* .. */
/* Have tracing here. */
/* .. */

/* Skip tracing from here. */
mollusk.invocation_inspect_callback = Box::new(EmptyInvocationInspectCallback {});
/* .. */

/* Have tracing back. */
mollusk.invocation_inspect_callback =
    Box::new(register_tracing::DefaultRegisterTracingCallback {
        sbf_trace_dir: std::env::var("SBF_TRACE_DIR").unwrap(),
    });
```
