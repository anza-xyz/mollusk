use {
    mollusk_svm::Mollusk,
    solana_account::Account,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    spl_token_2022_interface::{
        extension::{
            cpi_guard::CpiGuard,
            default_account_state::DefaultAccountState,
            group_member_pointer::GroupMemberPointer,
            group_pointer::GroupPointer,
            immutable_owner::ImmutableOwner,
            interest_bearing_mint::InterestBearingConfig,
            memo_transfer::MemoTransfer,
            metadata_pointer::MetadataPointer,
            mint_close_authority::MintCloseAuthority,
            non_transferable::{NonTransferable, NonTransferableAccount},
            pausable::{PausableAccount, PausableConfig},
            permanent_delegate::PermanentDelegate,
            permissioned_burn::PermissionedBurnConfig,
            scaled_ui_amount::ScaledUiAmountConfig,
            transfer_fee::{TransferFeeAmount, TransferFeeConfig},
            transfer_hook::{TransferHook, TransferHookAccount},
            BaseStateWithExtensions, BaseStateWithExtensionsMut, ExtensionType,
            StateWithExtensions, StateWithExtensionsMut,
        },
        state::{Account as TokenAccount, Mint},
    },
    spl_token_group_interface::state::{TokenGroup, TokenGroupMember},
    spl_token_metadata_interface::state::TokenMetadata,
};

pub const ID: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const ELF: &[u8] = include_bytes!("elf/token_2022.so");

/// Extension data for a Token-2022 Mint Account.
///
/// Confidential transfer extensions are not supported.
#[derive(Clone, Debug, PartialEq)]
pub enum MintExtension {
    TransferFeeConfig(TransferFeeConfig),
    MintCloseAuthority(MintCloseAuthority),
    DefaultAccountState(DefaultAccountState),
    NonTransferable,
    InterestBearingConfig(InterestBearingConfig),
    PermanentDelegate(PermanentDelegate),
    TransferHook(TransferHook),
    MetadataPointer(MetadataPointer),
    TokenMetadata(TokenMetadata),
    GroupPointer(GroupPointer),
    TokenGroup(TokenGroup),
    GroupMemberPointer(GroupMemberPointer),
    TokenGroupMember(TokenGroupMember),
    ScaledUiAmount(ScaledUiAmountConfig),
    Pausable(PausableConfig),
    PermissionedBurn(PermissionedBurnConfig),
}

impl MintExtension {
    fn extension_type(&self) -> ExtensionType {
        match self {
            Self::TransferFeeConfig(_) => ExtensionType::TransferFeeConfig,
            Self::MintCloseAuthority(_) => ExtensionType::MintCloseAuthority,
            Self::DefaultAccountState(_) => ExtensionType::DefaultAccountState,
            Self::NonTransferable => ExtensionType::NonTransferable,
            Self::InterestBearingConfig(_) => ExtensionType::InterestBearingConfig,
            Self::PermanentDelegate(_) => ExtensionType::PermanentDelegate,
            Self::TransferHook(_) => ExtensionType::TransferHook,
            Self::MetadataPointer(_) => ExtensionType::MetadataPointer,
            Self::TokenMetadata(_) => ExtensionType::TokenMetadata,
            Self::GroupPointer(_) => ExtensionType::GroupPointer,
            Self::TokenGroup(_) => ExtensionType::TokenGroup,
            Self::GroupMemberPointer(_) => ExtensionType::GroupMemberPointer,
            Self::TokenGroupMember(_) => ExtensionType::TokenGroupMember,
            Self::ScaledUiAmount(_) => ExtensionType::ScaledUiAmount,
            Self::Pausable(_) => ExtensionType::Pausable,
            Self::PermissionedBurn(_) => ExtensionType::PermissionedBurn,
        }
    }

    fn init(&self, state: &mut StateWithExtensionsMut<Mint>) {
        match self {
            Self::TransferFeeConfig(extension) => {
                *state.init_extension::<TransferFeeConfig>(true).unwrap() = *extension;
            }
            Self::MintCloseAuthority(extension) => {
                *state.init_extension::<MintCloseAuthority>(true).unwrap() = *extension;
            }
            Self::DefaultAccountState(extension) => {
                *state.init_extension::<DefaultAccountState>(true).unwrap() = *extension;
            }
            Self::NonTransferable => {
                state.init_extension::<NonTransferable>(true).unwrap();
            }
            Self::InterestBearingConfig(extension) => {
                *state.init_extension::<InterestBearingConfig>(true).unwrap() = *extension;
            }
            Self::PermanentDelegate(extension) => {
                *state.init_extension::<PermanentDelegate>(true).unwrap() = *extension;
            }
            Self::TransferHook(extension) => {
                *state.init_extension::<TransferHook>(true).unwrap() = *extension;
            }
            Self::MetadataPointer(extension) => {
                *state.init_extension::<MetadataPointer>(true).unwrap() = *extension;
            }
            Self::TokenMetadata(extension) => {
                state.init_variable_len_extension(extension, true).unwrap();
            }
            Self::GroupPointer(extension) => {
                *state.init_extension::<GroupPointer>(true).unwrap() = *extension;
            }
            Self::TokenGroup(extension) => {
                *state.init_extension::<TokenGroup>(true).unwrap() = *extension;
            }
            Self::GroupMemberPointer(extension) => {
                *state.init_extension::<GroupMemberPointer>(true).unwrap() = *extension;
            }
            Self::TokenGroupMember(extension) => {
                *state.init_extension::<TokenGroupMember>(true).unwrap() = *extension;
            }
            Self::ScaledUiAmount(extension) => {
                *state.init_extension::<ScaledUiAmountConfig>(true).unwrap() = *extension;
            }
            Self::Pausable(extension) => {
                *state.init_extension::<PausableConfig>(true).unwrap() = *extension;
            }
            Self::PermissionedBurn(extension) => {
                *state
                    .init_extension::<PermissionedBurnConfig>(true)
                    .unwrap() = *extension;
            }
        }
    }
}

/// Extension data for a Token-2022 Token Account.
///
/// Confidential transfer extensions are not supported.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenAccountExtension {
    TransferFeeAmount(TransferFeeAmount),
    ImmutableOwner,
    MemoTransfer(MemoTransfer),
    CpiGuard(CpiGuard),
    NonTransferableAccount,
    TransferHookAccount(TransferHookAccount),
    PausableAccount,
}

impl TokenAccountExtension {
    fn extension_type(&self) -> ExtensionType {
        match self {
            Self::TransferFeeAmount(_) => ExtensionType::TransferFeeAmount,
            Self::ImmutableOwner => ExtensionType::ImmutableOwner,
            Self::MemoTransfer(_) => ExtensionType::MemoTransfer,
            Self::CpiGuard(_) => ExtensionType::CpiGuard,
            Self::NonTransferableAccount => ExtensionType::NonTransferableAccount,
            Self::TransferHookAccount(_) => ExtensionType::TransferHookAccount,
            Self::PausableAccount => ExtensionType::PausableAccount,
        }
    }

    fn init(&self, state: &mut StateWithExtensionsMut<TokenAccount>) {
        match self {
            Self::TransferFeeAmount(extension) => {
                *state.init_extension::<TransferFeeAmount>(true).unwrap() = *extension;
            }
            Self::ImmutableOwner => {
                state.init_extension::<ImmutableOwner>(true).unwrap();
            }
            Self::MemoTransfer(extension) => {
                *state.init_extension::<MemoTransfer>(true).unwrap() = *extension;
            }
            Self::CpiGuard(extension) => {
                *state.init_extension::<CpiGuard>(true).unwrap() = *extension;
            }
            Self::NonTransferableAccount => {
                state
                    .init_extension::<NonTransferableAccount>(true)
                    .unwrap();
            }
            Self::TransferHookAccount(extension) => {
                *state.init_extension::<TransferHookAccount>(true).unwrap() = *extension;
            }
            Self::PausableAccount => {
                state.init_extension::<PausableAccount>(true).unwrap();
            }
        }
    }
}

pub fn add_program(mollusk: &mut Mollusk) {
    // Loader v3
    mollusk.add_program_with_loader_and_elf(
        &ID,
        &mollusk_svm::program::loader_keys::LOADER_V3,
        ELF,
    );
}

pub fn account() -> Account {
    // Loader v3
    mollusk_svm::program::create_program_account_loader_v3(&ID)
}

/// Get the key and account for the SPL Token-2022 program.
pub fn keyed_account() -> (Pubkey, Account) {
    (ID, account())
}

/// Create a Mint Account
pub fn create_account_for_mint(mint_data: Mint) -> Account {
    create_account_for_mint_with_extensions(mint_data, &[])
}

/// Create a Mint Account with extensions
pub fn create_account_for_mint_with_extensions(
    mint_data: Mint,
    extensions: &[MintExtension],
) -> Account {
    let fixed_len_types: Vec<ExtensionType> = extensions
        .iter()
        .filter(|extension| !matches!(extension, MintExtension::TokenMetadata(_)))
        .map(MintExtension::extension_type)
        .collect();

    let len = ExtensionType::try_calculate_account_len::<Mint>(&fixed_len_types).unwrap();
    let mut data = vec![0u8; len];

    {
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();
        for extension in extensions {
            if !matches!(extension, MintExtension::TokenMetadata(_)) {
                extension.init(&mut state);
            }
        }
    }

    // variable-length extensions require growing the buffer first
    for extension in extensions {
        if let MintExtension::TokenMetadata(metadata) = extension {
            let new_len = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data)
                .unwrap()
                .try_get_new_account_len_for_variable_len_extension(metadata)
                .unwrap();
            data.resize(new_len, 0);
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data)
                .unwrap()
                .init_variable_len_extension(metadata, true)
                .unwrap();
        }
    }

    let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();
    state.base = mint_data;
    state.pack_base();
    state.init_account_type().unwrap();

    Account {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a Token Account
pub fn create_account_for_token_account(token_account_data: TokenAccount) -> Account {
    create_account_for_token_account_with_extensions(token_account_data, &[])
}

/// Create a Token Account with extensions
pub fn create_account_for_token_account_with_extensions(
    token_account_data: TokenAccount,
    extensions: &[TokenAccountExtension],
) -> Account {
    let extension_types: Vec<ExtensionType> = extensions
        .iter()
        .map(TokenAccountExtension::extension_type)
        .collect();

    let len = ExtensionType::try_calculate_account_len::<TokenAccount>(&extension_types).unwrap();
    let mut data = vec![0u8; len];

    let mut state =
        StateWithExtensionsMut::<TokenAccount>::unpack_uninitialized(&mut data).unwrap();
    for extension in extensions {
        extension.init(&mut state);
    }
    state.base = token_account_data;
    state.pack_base();
    state.init_account_type().unwrap();

    Account {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Get the account extensions required by a mint's extensions
pub fn required_account_extensions_for_mint(
    mint_account_data: &[u8],
) -> Vec<TokenAccountExtension> {
    let state = StateWithExtensions::<Mint>::unpack(mint_account_data).unwrap();
    let mint_extension_types = state.get_extension_types().unwrap();
    ExtensionType::get_required_init_account_extensions(&mint_extension_types)
        .iter()
        .map(|extension_type| match extension_type {
            ExtensionType::TransferFeeAmount => {
                TokenAccountExtension::TransferFeeAmount(TransferFeeAmount::default())
            }
            ExtensionType::NonTransferableAccount => TokenAccountExtension::NonTransferableAccount,
            ExtensionType::TransferHookAccount => {
                TokenAccountExtension::TransferHookAccount(TransferHookAccount::default())
            }
            ExtensionType::PausableAccount => TokenAccountExtension::PausableAccount,
            _ => panic!("unsupported required extension: {extension_type:?}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        mollusk_svm::result::Check,
        solana_program_option::COption,
        solana_program_pack::Pack,
        spl_token_2022_interface::{
            extension::transfer_fee::TransferFee, instruction::mint_to, state::AccountState,
        },
    };

    fn mint_data(mint_authority: Pubkey) -> Mint {
        Mint {
            mint_authority: COption::Some(mint_authority),
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        }
    }

    fn token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> TokenAccount {
        TokenAccount {
            mint,
            owner,
            amount,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        }
    }

    fn transfer_fee_config(authority: Pubkey) -> TransferFeeConfig {
        let transfer_fee = TransferFee {
            epoch: 0.into(),
            maximum_fee: 5_000.into(),
            transfer_fee_basis_points: 100.into(),
        };
        TransferFeeConfig {
            transfer_fee_config_authority: Some(authority).try_into().unwrap(),
            withdraw_withheld_authority: Some(authority).try_into().unwrap(),
            withheld_amount: 0.into(),
            older_transfer_fee: transfer_fee,
            newer_transfer_fee: transfer_fee,
        }
    }

    #[test]
    fn no_extensions_matches_base_layout() {
        let account = create_account_for_mint(mint_data(Pubkey::new_unique()));
        assert_eq!(account.data.len(), Mint::LEN);

        let account = create_account_for_token_account(token_account_data(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
        ));
        assert_eq!(account.data.len(), TokenAccount::LEN);
    }

    #[test]
    fn mint_with_extensions() {
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let base = mint_data(authority);

        let config = transfer_fee_config(authority);
        let close = MintCloseAuthority {
            close_authority: Some(authority).try_into().unwrap(),
        };
        let metadata = TokenMetadata {
            update_authority: Some(authority).try_into().unwrap(),
            mint,
            name: "Mollusk".to_string(),
            symbol: "MLSK".to_string(),
            uri: "https://example.com".to_string(),
            additional_metadata: vec![],
        };

        let account = create_account_for_mint_with_extensions(
            base,
            &[
                MintExtension::TransferFeeConfig(config),
                MintExtension::MintCloseAuthority(close),
                MintExtension::NonTransferable,
                MintExtension::TokenMetadata(metadata.clone()),
            ],
        );

        let state = StateWithExtensions::<Mint>::unpack(&account.data).unwrap();
        assert_eq!(state.base, base);
        assert_eq!(*state.get_extension::<TransferFeeConfig>().unwrap(), config);
        assert_eq!(*state.get_extension::<MintCloseAuthority>().unwrap(), close);
        state.get_extension::<NonTransferable>().unwrap();
        assert_eq!(
            state.get_variable_len_extension::<TokenMetadata>().unwrap(),
            metadata
        );
    }

    #[test]
    fn token_account_with_extensions() {
        let owner = Pubkey::new_unique();
        let base = token_account_data(Pubkey::new_unique(), owner, 100);

        let cpi_guard = CpiGuard {
            lock_cpi: true.into(),
        };

        let account = create_account_for_token_account_with_extensions(
            base,
            &[
                TokenAccountExtension::ImmutableOwner,
                TokenAccountExtension::CpiGuard(cpi_guard),
            ],
        );

        let state = StateWithExtensions::<TokenAccount>::unpack(&account.data).unwrap();
        assert_eq!(state.base, base);
        state.get_extension::<ImmutableOwner>().unwrap();
        assert_eq!(*state.get_extension::<CpiGuard>().unwrap(), cpi_guard);
    }

    #[test]
    fn mint_to_with_transfer_fee() {
        let mut mollusk = Mollusk::default();
        add_program(&mut mollusk);

        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let token = Pubkey::new_unique();

        let mint_account = create_account_for_mint_with_extensions(
            mint_data(authority),
            &[MintExtension::TransferFeeConfig(transfer_fee_config(
                authority,
            ))],
        );
        let token_account = create_account_for_token_account_with_extensions(
            token_account_data(mint, owner, 0),
            &required_account_extensions_for_mint(&mint_account.data),
        );

        let instruction = mint_to(&ID, &mint, &token, &authority, &[], 1_000).unwrap();
        let result = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (mint, mint_account),
                (token, token_account),
                (authority, Account::default()),
            ],
            &[Check::success()],
        );

        let resulting_token_account = result.get_account(&token).unwrap();
        let state =
            StateWithExtensions::<TokenAccount>::unpack(&resulting_token_account.data).unwrap();
        assert_eq!(state.base.amount, 1_000);
        state.get_extension::<TransferFeeAmount>().unwrap();
    }
}
