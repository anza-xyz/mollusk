<p align="center">
  <a href="https://solana.com">
    <img alt="Solana" src="https://github.com/user-attachments/assets/534af75d-6347-48dc-8943-129423b2ba63" height="80" />
  </a>
</p>

# Solana System Interface

This crate contains instructions and constructors for interacting with the [System program](https://solana.com/docs/core/programs#core-programs).

The System program can be used to create new accounts, allocate account data, assign accounts to owning programs, transfer lamports from System Program owned accounts and pay transaction fees.

## Getting Started

From your project folder:

```bash
cargo add solana-system-interface --features bincode
```

This will add the `solana-system-interface` dependency with the `bincode` feature enabled to your `Cargo.toml` file. The `bincode` feature contains the instruction constructors to create instructions for the System program.

## Examples

Creating an account:

```rust
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction;
use anyhow::Result;

fn create_account(
    client: &RpcClient,
    payer: &Keypair,
    new_account: &Keypair,
    owning_program: &Pubkey,
    space: u64,
) -> Result<()> {
    let rent = client.get_minimum_balance_for_rent_exemption(space.try_into()?)?;
    let instr = instruction::create_account(
        &payer.pubkey(),
        &new_account.pubkey(),
        rent,
        space,
        owning_program,
    );

    let blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[instr],
        Some(&payer.pubkey()),
        &[payer, new_account],
        blockhash,
    );

    let _sig = client.send_and_confirm_transaction(&tx)?;

    Ok(())
}
```

Transfer lamports between accounts:

```rust
use solana_rpc_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction;
use anyhow::Result;

fn transfer(
    client: &RpcClient,
    from: &Keypair,
    recipient: &Pubkey,
    lamports: u64,
) -> Result<()> {
    let instr = instruction::transfer(
        &from.pubkey(),
        recipient,
        lamports,
    );

    let blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[instr],
        Some(&from.pubkey()),
        &[from],
        blockhash,
    );

    let _sig = client.send_and_confirm_transaction(&tx)?;

    Ok(())
}
```

More examples can be found on the crate [documentation](https://docs.rs/solana-system-interface/latest/solana-system-interface/).
