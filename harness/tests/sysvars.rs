#![cfg(feature = "invocation-inspect-callback")]

use {
    mollusk_svm::{InvocationInspectCallback, Mollusk},
    solana_account::Account,
    solana_instruction::Instruction,
    solana_program_runtime::invoke_context::InvokeContext,
    solana_pubkey::Pubkey,
    solana_transaction_context::instruction_accounts::InstructionAccount,
    std::{cell::Cell, rc::Rc},
};

/// Records the clock slot the runtime saw during the last invocation.
struct ClockSlotRecorder(Rc<Cell<Option<u64>>>);

impl InvocationInspectCallback for ClockSlotRecorder {
    fn before_invocation(
        &self,
        _: &Mollusk,
        _: &Pubkey,
        _: &[u8],
        _: &[InstructionAccount],
        invoke_context: &mut InvokeContext,
        _: bool,
    ) {
        let clock = invoke_context
            .environment_config
            .sysvar_cache()
            .get_clock()
            .unwrap();
        self.0.set(Some(clock.slot));
    }

    fn after_invocation(&self, _: &Mollusk, _: &InvokeContext, _: bool) {}
}

fn setup() -> (Mollusk, Rc<Cell<Option<u64>>>) {
    let seen_slot = Rc::new(Cell::new(None));
    let mut mollusk = Mollusk::default();
    mollusk.invocation_inspect_callback = Box::new(ClockSlotRecorder(Rc::clone(&seen_slot)));
    (mollusk, seen_slot)
}

fn transfer() -> (Instruction, Vec<(Pubkey, Account)>) {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, 1);
    let accounts = vec![
        (
            sender,
            Account::new(1_000_000_000, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::new(1_000_000_000, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];
    (instruction, accounts)
}

fn executed_slot(
    mollusk: &Mollusk,
    seen_slot: &Cell<Option<u64>>,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
) -> u64 {
    seen_slot.set(None);
    mollusk.process_instruction(instruction, accounts);
    seen_slot.get().expect("callback didn't run")
}

#[test]
fn test_set_sysvars_updates_executed_clock() {
    let (mut mollusk, seen_slot) = setup();
    let (instruction, accounts) = transfer();
    let initial_slot = mollusk.sysvars().clock.slot;
    assert_eq!(
        executed_slot(&mollusk, &seen_slot, &instruction, &accounts),
        initial_slot
    );

    let mut sysvars = mollusk.sysvars().clone();
    sysvars.clock.slot = initial_slot + 42;
    mollusk.set_sysvars(sysvars);

    assert_eq!(
        executed_slot(&mollusk, &seen_slot, &instruction, &accounts),
        initial_slot + 42
    );
}

#[test]
fn test_warp_to_slot_updates_executed_clock() {
    let (mut mollusk, seen_slot) = setup();
    let (instruction, accounts) = transfer();

    mollusk.warp_to_slot(1_000);

    assert_eq!(mollusk.sysvars().clock.slot, 1_000);
    assert_eq!(
        executed_slot(&mollusk, &seen_slot, &instruction, &accounts),
        1_000
    );
}

#[test]
fn test_sysvar_account_overrides_cached_clock() {
    let (mollusk, seen_slot) = setup();
    let (instruction, mut accounts) = transfer();
    let cached_slot = mollusk.sysvars().clock.slot;

    // A clock account passed with the instruction takes precedence.
    let mut sysvars = mollusk.sysvars().clone();
    sysvars.clock.slot = cached_slot + 7;
    accounts.push(sysvars.keyed_account_for_clock_sysvar());
    assert_eq!(
        executed_slot(&mollusk, &seen_slot, &instruction, &accounts),
        cached_slot + 7
    );

    // Without it, the cached sysvars are used again.
    accounts.pop();
    assert_eq!(
        executed_slot(&mollusk, &seen_slot, &instruction, &accounts),
        cached_slot
    );
}
