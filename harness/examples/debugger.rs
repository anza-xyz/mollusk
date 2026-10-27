//! Walkthrough driver for the SBPF debugger.
//!
//! Executes an instruction on `test_program_primary` that CPIs into
//! `test_program_cpi_target`. With `SBF_DEBUG_PORT` set, each of the two
//! program invocations pauses on a GDB remote stub until a debugger attaches
//! and continues it.
//!
//! Run `make debugger-demo` from the repository root and follow the
//! "Interactive Debugging" section of the README.

use {
    mollusk_svm::{
        program::{create_program_account_loader_v3, loader_keys::LOADER_V3},
        result::Check,
        Mollusk,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
};

fn main() {
    let program_id = Pubkey::from_str_const("239vxAL9Q7e3uLoinJpJ873r3bvT9sPFxH7yekwPppNF");
    let cpi_target_program_id =
        Pubkey::from_str_const("MD24T7azhc2q9ZXaeskbLpmVA41k7StzTGgcfvGcpHj");

    // Register tracing must be enabled for the debugger to engage. The
    // default callback picks up `SBF_DEBUG_PORT` from the environment.
    let mut mollusk = Mollusk::new_debuggable(
        &program_id,
        "test_program_primary",
        /* enable_register_tracing */ true,
    );
    mollusk.add_program_with_loader(
        &cpi_target_program_id,
        "test_program_cpi_target",
        &LOADER_V3,
    );

    let data = b"mollusk";
    let key = Pubkey::new_unique();
    let account = Account::new(
        mollusk.sysvars.rent.minimum_balance(data.len()),
        data.len(),
        &cpi_target_program_id,
    );

    // Instruction 4: CPI into the target program, which writes `data` into
    // the first account.
    let mut instruction_data = vec![4];
    instruction_data.extend_from_slice(cpi_target_program_id.as_ref());
    instruction_data.extend_from_slice(data);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(key, true),
            AccountMeta::new_readonly(cpi_target_program_id, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (key, account),
            (
                cpi_target_program_id,
                create_program_account_loader_v3(&cpi_target_program_id),
            ),
        ],
        &[Check::success(), Check::account(&key).data(data).build()],
    );

    println!("Instruction succeeded; account data is {data:?}");
}
