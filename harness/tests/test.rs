use std::sync::Arc;

use mollusk_svm::{program, Mollusk};
use once_cell::sync::Lazy;
use solana_account::{Account, AccountSharedData, ReadableAccount};
use solana_client::rpc_client::RpcClient;
// use solana_client::nonblocking::rpc_client::RpcClient;
use solana_instruction::{AccountMeta, BorrowedAccountMeta, BorrowedInstruction, Instruction};
use solana_program::sysvar::instructions::construct_instructions_data;
use solana_pubkey::{pubkey, Pubkey};
use solana_transaction_context::{InstructionAccount, TransactionContext};
use spl_token_2022::{extension::StateWithExtensions, state::Account as SplAccount};



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
    mollusk.add_program(
        &SOL_FI,
        "/ssd1/mnt/program/solfi",
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

    let accounts = [
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
        accounts.to_vec()
    );
    let instruction_accounts = [
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
        StateWithExtensions::<SplAccount>::unpack(user_token_source_account.data.as_ref()).unwrap().base;
    let befer_token_destination = 
        StateWithExtensions::<SplAccount>::unpack(user_token_destination_account.data.as_ref()).unwrap().base;
    
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
                let after_token_destination = StateWithExtensions::<SplAccount>::unpack(a.data().as_ref()).unwrap().base;
                println!("user_token_destination: {:?}", after_token_destination.amount-befer_token_destination.amount);
            },
            None => println!("user_token_destination not found"),
        }
        match transaction_context.find_index_of_account(&user_token_source) {
            Some(index) => {
                let a = transaction_context.get_account_at_index(index).unwrap().borrow();
                let after_token_source = StateWithExtensions::<SplAccount>::unpack(a.data().as_ref()).unwrap().base;
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
    for i in 0..10 {
        let befer_token_source = 
            StateWithExtensions::<SplAccount>::unpack(user_token_source_account.data.as_ref()).unwrap().base;
        let befer_token_destination = 
            StateWithExtensions::<SplAccount>::unpack(user_token_destination_account.data.as_ref()).unwrap().base;

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
                let after_token_source = StateWithExtensions::<SplAccount>::unpack(a.data.as_ref()).unwrap().base;
                println!("User token source: {:?}", befer_token_source.amount-after_token_source.amount);
            } else if *pk == user_token_destination {
                let after_token_destination = StateWithExtensions::<SplAccount>::unpack(a.data.as_ref()).unwrap().base;
                println!("user_token_destination: {:?}", after_token_destination.amount-befer_token_destination.amount);
            }
        }
    }


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
  
    println!("");
    // println!("Result: {:?}", res);
}




pub const USER: Pubkey = pubkey!("3nMNd89AxwHUa1AFvQGqohRkxFEQsTsgiEyEyqXFHyyH");
pub const SPL_ASSOCIATED_TOKEN_ACCOUNT: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const IXS_SYSVAR: Pubkey =  pubkey!("Sysvar1nstructions1111111111111111111111111");
pub const SOL_FI: Pubkey = pubkey!("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
pub const SYSVAR: Pubkey =  pubkey!("Sysvar1111111111111111111111111111111111111");

#[test]
fn solfi_test() {
    let pool_address = pubkey!("CAPhoEse9xEH95XmdnJjYrZdNCA8xfUWdy3aWymHa1Vj");

    let client = RpcClient::new(RPC_URL.to_owned());
    let pool_account = client.get_account(&pool_address).unwrap();
    let mint0 = Pubkey::new_from_array(pool_account.data[2664..2696].try_into().unwrap());
    let mint1 = Pubkey::new_from_array(pool_account.data[2696..2728].try_into().unwrap());
    let pool0 = Pubkey::new_from_array(pool_account.data[2736..2768].try_into().unwrap());
    let pool1 = Pubkey::new_from_array(pool_account.data[2768..2800].try_into().unwrap());
    
    let account_metas = vec![
        AccountMeta::new(USER, true),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(pool0, false),
        AccountMeta::new(pool1, false),
        AccountMeta::new(Pubkey::find_program_address(
            &[&USER.as_ref(), &TOKEN_PROGRAM.as_ref(), mint0.as_ref()],
                &SPL_ASSOCIATED_TOKEN_ACCOUNT).0, false),
        AccountMeta::new(Pubkey::find_program_address(
            &[&USER.as_ref(), &TOKEN_PROGRAM.as_ref(), mint1.as_ref()],
                &SPL_ASSOCIATED_TOKEN_ACCOUNT).0, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(IXS_SYSVAR, false),
    ];
    let amount_in = 1_000_000_0u64;
    let from = pubkey!("So11111111111111111111111111111111111111112");
    let to = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    let mut instruction_data = [0u8; 18];
    instruction_data[0] = 7;
    instruction_data[1..9].copy_from_slice(&amount_in.to_le_bytes());
    
    if from != mint0 {
        instruction_data[17] = 1;
    };
    let ins = Instruction {
            program_id: SOL_FI,
            accounts: account_metas.clone(),
            data: instruction_data.to_vec(),
        };
    let mut accounts = Vec::new();
    let pks = ins.accounts
        .iter()
        .map(|a|a.pubkey)
        .collect::<Vec<Pubkey>>()[0..6].to_vec();

    let ad = client.get_multiple_accounts(&pks).unwrap();

    let mut dst_index = 5;
    if ins.data[17] == 1 {
        dst_index = 4;
    }        
    let dsp_pk = pks[dst_index];
    let befer_dst = u64::from_le_bytes(ad[dst_index].as_ref().unwrap().data[64..72].try_into().unwrap());
    println!("befer: {}", befer_dst);
    for i in 0..6 {
        accounts.push((ins.accounts[i].pubkey, ad[i].clone().unwrap()));
    }
    accounts.push((TOKEN_PROGRAM, program::create_program_account_loader_v3(&TOKEN_PROGRAM)));
    
    let data = construct_instructions_data(&[
        BorrowedInstruction {
            program_id: &ins.program_id,
            accounts: ins.accounts.iter().map(|a|BorrowedAccountMeta {
                pubkey: &a.pubkey,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }).collect::<Vec<BorrowedAccountMeta>>(),
            data: &ins.data,
        }
    ]);
    // println!("{:?}", data);
    accounts.push((IXS_SYSVAR, Account {
        data: data,
        owner: SYSVAR,
        ..Account::default()
    }));
    
    let res = MOLLUSK.process_instruction(
        &ins,
        &accounts,
    );
    for (pk, a) in res.resulting_accounts.iter() {
        if *pk == dsp_pk {
            let after = u64::from_le_bytes(a.data[64..72].try_into().unwrap());
            println!("user_token_destination: {:?}", after-befer_dst);
        }
    }
    println!("{:?}", res.raw_result);
}

#[test]
fn solfi_test_2() {
    let pool_address = pubkey!("CAPhoEse9xEH95XmdnJjYrZdNCA8xfUWdy3aWymHa1Vj");

    let client = RpcClient::new(RPC_URL.to_owned());
    let pool_account = client.get_account(&pool_address).unwrap();
    let mint0 = Pubkey::new_from_array(pool_account.data[2664..2696].try_into().unwrap());
    let mint1 = Pubkey::new_from_array(pool_account.data[2696..2728].try_into().unwrap());
    let pool0 = Pubkey::new_from_array(pool_account.data[2736..2768].try_into().unwrap());
    let pool1 = Pubkey::new_from_array(pool_account.data[2768..2800].try_into().unwrap());
    
    let account_metas = vec![
        AccountMeta::new(USER, true),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(pool0, false),
        AccountMeta::new(pool1, false),
        AccountMeta::new(Pubkey::find_program_address(
            &[&USER.as_ref(), &TOKEN_PROGRAM.as_ref(), mint0.as_ref()],
                &SPL_ASSOCIATED_TOKEN_ACCOUNT).0, false),
        AccountMeta::new(Pubkey::find_program_address(
            &[&USER.as_ref(), &TOKEN_PROGRAM.as_ref(), mint1.as_ref()],
                &SPL_ASSOCIATED_TOKEN_ACCOUNT).0, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(IXS_SYSVAR, false),
    ];
    let amount_in = 1_000_000_0u64;
    let from = pubkey!("So11111111111111111111111111111111111111112");
    let to = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    let mut instruction_data = [0u8; 18];
    instruction_data[0] = 7;
    instruction_data[1..9].copy_from_slice(&amount_in.to_le_bytes());
    
    if from != mint0 {
        instruction_data[17] = 1;
    };
    let ins = Instruction {
        program_id: SOL_FI,
        accounts: account_metas.clone(),
        data: instruction_data.to_vec(),
    };
    // let mut accounts = Vec::new();
    let pks = ins.accounts
        .iter()
        .map(|a|a.pubkey)
        .collect::<Vec<Pubkey>>()[0..6].to_vec();

    let ad = client.get_multiple_accounts(&pks).unwrap();

    let mut dst_index = 5;
    if ins.data[17] == 1 {
        dst_index = 4;
    }        
    let dsp_pk = pks[dst_index];
    let befer_dst = u64::from_le_bytes(ad[dst_index].as_ref().unwrap().data[64..72].try_into().unwrap());
    println!("befer: {}", befer_dst);
    let (token_program, token_account) = (
        TOKEN_PROGRAM,
        program::create_program_account_loader_v3(&TOKEN_PROGRAM),
    );
    let instruction_accounts = [
        InstructionAccount { index_in_transaction: 2, index_in_caller: 2, index_in_callee: 0, is_signer: true,  is_writable: true },
        InstructionAccount { index_in_transaction: 4, index_in_caller: 4, index_in_callee: 1, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 3, index_in_caller: 3, index_in_callee: 2, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 7, index_in_caller: 7, index_in_callee: 3, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 0, index_in_caller: 0, index_in_callee: 4, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 1, index_in_caller: 1, index_in_callee: 5, is_signer: false, is_writable: true },
        InstructionAccount { index_in_transaction: 8, index_in_caller: 8, index_in_callee: 6, is_signer: false, is_writable: false },
        InstructionAccount { index_in_transaction: 5, index_in_caller: 5, index_in_callee: 7, is_signer: false, is_writable: false },
    ];
    let data = construct_instructions_data(&[
        BorrowedInstruction {
            program_id: &ins.program_id,
            accounts: ins.accounts.iter().map(|a|BorrowedAccountMeta {
                pubkey: &a.pubkey,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }).collect::<Vec<BorrowedAccountMeta>>(),
            data: &ins.data,
        }
    ]);
    let program_id_index = 6;
    let transaction_accounts = vec![
        (account_metas[4].pubkey, form_asd(ad[4].clone().unwrap())),
        (account_metas[5].pubkey, form_asd(ad[5].clone().unwrap())),
        (account_metas[0].pubkey, form_asd(ad[0].clone().unwrap())),
        (account_metas[2].pubkey, form_asd(ad[2].clone().unwrap())),
        (account_metas[1].pubkey, form_asd(ad[1].clone().unwrap())),
        (account_metas[7].pubkey, form_asd(Account {
            data: data,
            owner: SYSVAR,
            ..Account::default()
        })),
        (SOL_FI,  AccountSharedData {
            lamports: 0,
            data: Arc::new(vec![]),
            owner: pubkey!("BPFLoaderUpgradeab1e11111111111111111111111"),
            executable: true,
            rent_epoch: 0,
        }),
        (account_metas[3].pubkey, form_asd(ad[3].clone().unwrap())),
        (token_program, form_asd(token_account)),
    ];

    let sysvar_cache = MOLLUSK.sysvars.setup_sysvar_cache_v2(&transaction_accounts);
    let transaction_context = TransactionContext::new(
        transaction_accounts,
        MOLLUSK.sysvars.rent.clone(),
        MOLLUSK.compute_budget.max_instruction_stack_depth,
        MOLLUSK.compute_budget.max_instruction_trace_length,
    );
    let mut transaction_context = transaction_context.clone();
    let mut compute_units_consumed = 0;
    MOLLUSK.process_instruction_v2(
        &sysvar_cache,
        &mut transaction_context,
        &instruction_accounts,
        &ins.data,
        program_id_index,
        &mut compute_units_consumed
    ).unwrap();
    match transaction_context.find_index_of_account(&dsp_pk) {
        Some(index) => {
            let a = transaction_context.get_account_at_index(index).unwrap().borrow();
            let after_token_destination = StateWithExtensions::<SplAccount>::unpack(a.data().as_ref()).unwrap().base;
            println!("user_token_destination: {:?}", after_token_destination.amount-befer_dst);
        },
        None => println!("user_token_destination not found"),
    }
}