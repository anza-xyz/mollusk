use std::sync::Arc;

use mollusk_svm::{program, Mollusk};
use once_cell::sync::Lazy;
use solana_account::{AccountSharedData, ReadableAccount};
use solana_client::rpc_client::RpcClient;
// use solana_client::nonblocking::rpc_client::RpcClient;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::{pubkey, Pubkey};
use solana_transaction_context::{InstructionAccount, TransactionContext};
use spl_token_2022::{extension::StateWithExtensions, state::Account};



pub const RAYDIUM_AMM: Pubkey =  pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
// pub const RPC_URL: &str = "http://192.168.31.122:8899";
pub const RPC_URL: &str = "https://solana-mainnet.core.chainstack.com/5fb086391c34ff12377fc008d6afc63f";



pub static MOLLUSK: Lazy<Mollusk> = Lazy::new(|| {
    let mut mollusk = Mollusk::new(
        &RAYDIUM_AMM, 
        "/ssd1/mnt/program/raydium_amm"
    );
    mollusk.add_program(
        &TOKEN_PROGRAM,
        "/ssd1/mollusk/spl_token",
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );
    mollusk
});
// solana program dump -u https://solana-mainnet.core.chainstack.com/5fb086391c34ff12377fc008d6afc63f TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA /ssd1/mollusk/spl_token.so

// cargo test --release --package mollusk-svm --test test -- test_2 --exact --show-output

pub fn form_asd(other: solana_account::Account) -> AccountSharedData {
    AccountSharedData {
        lamports: other.lamports,
        data: Arc::new(other.data),
        owner: other.owner,
        executable: other.executable,
        rent_epoch: other.rent_epoch,
    }
}
#[test]
fn test_2() {

    let client = RpcClient::new(RPC_URL.to_owned());
    let pks = [
        pubkey!("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"),
        pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1"),
        pubkey!("DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSb9JtteuvPpz"),
        pubkey!("HLmqeL62xR1QoZ1HKKbXRrdN1p3phKpxRMb2VVopvBBz"),
        pubkey!("AthF4CEubMxyPVkw8PKbLsPgs9sj6uRHLskGLuBybMpS"),
        pubkey!("HNoyj2rSfaKyV8Aynzp81DdUrwMyMxLN5649Fz7Ty77D"),
        pubkey!("3nMNd89AxwHUa1AFvQGqohRkxFEQsTsgiEyEyqXFHyyH")
    ];

    let accounts = client.get_multiple_accounts(&pks).unwrap();

    let pool_address = pks[0];
    let pool_address_account = accounts[0].clone().unwrap();

    let raydium_authority_v4 = pks[1];
    let raydium_authority_v4_account = accounts[1].clone().unwrap();

    let coin_vault = pks[2];
    let coin_vault_account = accounts[2].clone().unwrap();

    let pc_vault = pks[3];
    let pc_vault_account = accounts[3].clone().unwrap();

    let user_token_source = pks[4];
    let user_token_source_account = accounts[4].clone().unwrap();

    let user_token_destination = pks[5];
    let user_token_destination_account = accounts[5].clone().unwrap();

    let user = pks[6];
    let user_account = accounts[6].clone().unwrap();

    let (token_program, token_account) = (
        TOKEN_PROGRAM,
        program::create_program_account_loader_v3(&TOKEN_PROGRAM),
    );
    // let mollusk = mollusk();

    let accounts = vec![
        // spl token
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        // amm
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(raydium_authority_v4, false),
        AccountMeta::new_readonly(pool_address, false),
        // AccountMeta::new(amm_info.open_orders, false),
        AccountMeta::new(coin_vault, false),
        AccountMeta::new(pc_vault, false),
        // market
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        // user
        AccountMeta::new(user_token_source, false),
        AccountMeta::new(user_token_destination, false),
        AccountMeta::new(user, true),
    ];
    let mut data = Vec::new();
    data.push(9);
    data.extend_from_slice(&100_000_0u64.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        RAYDIUM_AMM,
        &data,
        accounts,
    );
    let instruction_accounts = vec![
        InstructionAccount { index_in_transaction: 7, index_in_caller: 7, index_in_callee: 0, is_signer: false, is_writable: false },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 0, index_in_caller: 0, index_in_callee: 2, is_signer: false, is_writable: false },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 6, index_in_caller: 6, index_in_callee: 4, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 5, index_in_caller: 5, index_in_callee: 5, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 8, index_in_caller: 8, index_in_callee: 14, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 1, index_in_caller: 1, index_in_callee: 15, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 4, index_in_caller: 4, index_in_callee: 16, is_signer: true, is_writable: true },
    ];
    
    
    let befer_token_source = 
        StateWithExtensions::<Account>::unpack(user_token_source_account.data.as_ref()).unwrap().base;
    let befer_token_destination = 
        StateWithExtensions::<Account>::unpack(user_token_destination_account.data.as_ref()).unwrap().base;
    
    let program_id_index = 2;
    let transaction_accounts = vec![
        (raydium_authority_v4, form_asd(raydium_authority_v4_account)),
        (user_token_destination, form_asd(user_token_destination_account)),
        (RAYDIUM_AMM,  AccountSharedData {
            lamports: 0,
            data: Arc::new(vec![]),
            owner: pubkey!("BPFLoaderUpgradeab1e11111111111111111111111"),
            executable: true,
            rent_epoch: 0,
        }),
        (pool_address, form_asd(pool_address_account)),
        (user, form_asd(user_account)),
        (pc_vault, form_asd(pc_vault_account)),
        (coin_vault, form_asd(coin_vault_account)),
        (token_program, form_asd(token_account)),
        (user_token_source, form_asd(user_token_source_account)),
    ];

    let sysvar_cache = MOLLUSK.sysvars.setup_sysvar_cache_v2(&transaction_accounts);
    let transaction_context = TransactionContext::new(
        transaction_accounts,
        MOLLUSK.sysvars.rent.clone(),
        MOLLUSK.compute_budget.max_instruction_stack_depth,
        MOLLUSK.compute_budget.max_instruction_trace_length,
    );
    for i in 0..10 {

        println!("test instruction {}", i);
        let s = std::time::Instant::now();
        let mut transaction_context = transaction_context.clone();
        let mut compute_units_consumed = 0;
        MOLLUSK.process_instruction_v2(
            &sysvar_cache,
            &mut transaction_context,
            &instruction_accounts,
            &instruction.data,
            program_id_index,
            &mut compute_units_consumed
        ).unwrap();
        println!("Time: {:?}", s.elapsed());

        println!("test instruction {}", i);
        println!("compute_units_consumed: {}", compute_units_consumed);
        match transaction_context.find_index_of_account(&user_token_destination) {
            Some(index) => {
                let a = transaction_context.get_account_at_index(index).unwrap().borrow();
                let after_token_destination = StateWithExtensions::<Account>::unpack(a.data().as_ref()).unwrap().base;
                println!("user_token_destination: {:?}", after_token_destination.amount-befer_token_destination.amount);
            },
            None => println!("user_token_destination not found"),
        }
        match transaction_context.find_index_of_account(&user_token_source) {
            Some(index) => {
                let a = transaction_context.get_account_at_index(index).unwrap().borrow();
                let after_token_source = StateWithExtensions::<Account>::unpack(a.data().as_ref()).unwrap().base;
                println!("user_token_source: {:?}", befer_token_source.amount-after_token_source.amount);
            },
            None => println!("user_token_source not found"),
        }
    }


    println!("");
    // println!("Result: {:?}", res);
}




pub fn mollusk() -> Mollusk {
    let mut mollusk = Mollusk::new(
        &RAYDIUM_AMM, 
        "/ssd1/mnt/program/raydium_amm"
        // "/ssd1/contract-test/target/sbpf-solana-solana/release/contract_test"
    );
    mollusk.add_program(
        &TOKEN_PROGRAM,
        "/ssd1/mollusk/spl_token",
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );
    mollusk
}
#[test]
fn test() {

    let client = RpcClient::new(RPC_URL.to_owned());
    let pks = [
        pubkey!("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"),
        pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1"),
        pubkey!("DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSb9JtteuvPpz"),
        pubkey!("HLmqeL62xR1QoZ1HKKbXRrdN1p3phKpxRMb2VVopvBBz"),
        pubkey!("AthF4CEubMxyPVkw8PKbLsPgs9sj6uRHLskGLuBybMpS"),
        pubkey!("HNoyj2rSfaKyV8Aynzp81DdUrwMyMxLN5649Fz7Ty77D"),
        pubkey!("3nMNd89AxwHUa1AFvQGqohRkxFEQsTsgiEyEyqXFHyyH")
    ];

    let accounts = client.get_multiple_accounts(&pks).unwrap();

    let pool_address = pks[0];
    let pool_address_account = accounts[0].clone().unwrap();

    let raydium_authority_v4 = pks[1];
    let raydium_authority_v4_account = accounts[1].clone().unwrap();

    let coin_vault = pks[2];
    let coin_vault_account = accounts[2].clone().unwrap();

    let pc_vault = pks[3];
    let pc_vault_account = accounts[3].clone().unwrap();

    let user_token_source = pks[4];
    let user_token_source_account = accounts[4].clone().unwrap();

    let user_token_destination = pks[5];
    let user_token_destination_account = accounts[5].clone().unwrap();

    let user = pks[6];
    let user_account = accounts[6].clone().unwrap();

    let (token_program, token_account) = (
        TOKEN_PROGRAM,
        program::create_program_account_loader_v3(&TOKEN_PROGRAM),
    );
    let mollusk = mollusk();

    let accounts = vec![
        // spl token
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        // amm
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(raydium_authority_v4, false),
        AccountMeta::new_readonly(pool_address, false),
        // AccountMeta::new(amm_info.open_orders, false),
        AccountMeta::new(coin_vault, false),
        AccountMeta::new(pc_vault, false),
        // market
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pool_address, false),
        // user
        AccountMeta::new(user_token_source, false),
        AccountMeta::new(user_token_destination, false),
        AccountMeta::new(user, true),
    ];
    let mut data = Vec::new();
    data.push(9);
    data.extend_from_slice(&100_000_0u64.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        RAYDIUM_AMM,
        &data,
        accounts,
    );


    // for i in 0..1 {
    //     let befer_token_source = 
    //         StateWithExtensions::<Account>::unpack(user_token_source_account.data.as_ref()).unwrap().base;
    //     let befer_token_destination = 
    //         StateWithExtensions::<Account>::unpack(user_token_destination_account.data.as_ref()).unwrap().base;

    //     println!("test instruction {}", i);
    //     let accounts = [
    //         (token_program, token_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (raydium_authority_v4, raydium_authority_v4_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (coin_vault, coin_vault_account.clone()),
    //         (pc_vault, pc_vault_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (pool_address, pool_address_account.clone()),
    //         (user_token_source, user_token_source_account.clone()),
    //         (user_token_destination, user_token_destination_account.clone()),
    //         (user, user_account.clone())
    //     ];
    //     let s = std::time::Instant::now();

    //     let (
    //         sysvar_cache,
    //         transaction_context,
    //         instruction_accounts,
    //         program_id_index
    //     ) = mollusk.data_prepare(&instruction, &accounts);

    //     println!("data_prepare: {:?}", s.elapsed());

    //     let s = std::time::Instant::now();
    //     let mut transaction_context = transaction_context.clone();
    //     let mut compute_units_consumed = 0;
    //     mollusk.process_instruction_v2(
    //         &sysvar_cache,
    //         &mut transaction_context,
    //         &instruction_accounts,
    //         &instruction.data,
    //         program_id_index,
    //         &mut compute_units_consumed
    //     ).unwrap();
    //     println!("Time: {:?}", s.elapsed());
    //     println!("test instruction {}", i);
    //     println!("compute_units_consumed: {}", compute_units_consumed);
    //     match transaction_context.find_index_of_account(&user_token_destination) {
    //         Some(index) => {
    //             let a = transaction_context.get_account_at_index(index).unwrap().borrow();
    //             let after_token_destination = StateWithExtensions::<Account>::unpack(a.data().as_ref()).unwrap().base;
    //             println!("user_token_destination: {:?}", after_token_destination.amount-befer_token_destination.amount);
    //         },
    //         None => println!("user_token_destination not found"),
    //     }
    //     match transaction_context.find_index_of_account(&user_token_source) {
    //         Some(index) => {
    //             let a = transaction_context.get_account_at_index(index).unwrap().borrow();
    //             let after_token_source = StateWithExtensions::<Account>::unpack(a.data().as_ref()).unwrap().base;
    //             println!("user_token_source: {:?}", befer_token_source.amount-after_token_source.amount);
    //         },
    //         None => println!("user_token_source not found"),
    //     }
    // }
    for i in 0..10 {
        let befer_token_source = 
            StateWithExtensions::<Account>::unpack(user_token_source_account.data.as_ref()).unwrap().base;
        let befer_token_destination = 
            StateWithExtensions::<Account>::unpack(user_token_destination_account.data.as_ref()).unwrap().base;

        println!("test instruction {}", i);
        let accounts = [
            (token_program, token_account.clone()),
            (pool_address, pool_address_account.clone()),
            (raydium_authority_v4, raydium_authority_v4_account.clone()),
            (pool_address, pool_address_account.clone()),
            (coin_vault, coin_vault_account.clone()),
            (pc_vault, pc_vault_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (pool_address, pool_address_account.clone()),
            (user_token_source, user_token_source_account.clone()),
            (user_token_destination, user_token_destination_account.clone()),
            (user, user_account.clone())
        ];
        let s = std::time::Instant::now();
        let res = mollusk.process_instruction(
            &instruction,
            &accounts,
            // &[Check::success()],
        );
        println!("Time: {:?}", s.elapsed());
        println!("test instruction {}", i);
        println!("res: {:?}", res.compute_units_consumed);
        for (pk, a) in res.resulting_accounts.iter() {
            if *pk == user_token_source {
                let after_token_source = StateWithExtensions::<Account>::unpack(a.data.as_ref()).unwrap().base;
                println!("User token source: {:?}", befer_token_source.amount-after_token_source.amount);
            } else if *pk == user_token_destination {
                let after_token_destination = StateWithExtensions::<Account>::unpack(a.data.as_ref()).unwrap().base;
                println!("user_token_destination: {:?}", after_token_destination.amount-befer_token_destination.amount);
            }
        }
    }

    println!("");
    // println!("Result: {:?}", res);
}


