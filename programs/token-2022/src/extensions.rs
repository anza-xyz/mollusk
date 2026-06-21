//! Extension-aware account builders for SPL Token-2022 (`extensions` feature).

use {crate::rent_exempt_account, solana_account::Account};
pub use {
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
        state::{Account as TokenAccount, AccountState, Mint},
    },
    spl_token_group_interface::state::{TokenGroup, TokenGroupMember},
    spl_token_metadata_interface::state::TokenMetadata,
};

/// Extension data for a Token-2022 Mint Account.
///
/// Confidential transfer extensions are not supported.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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

    // Sized by growing the buffer, not `try_calculate_account_len`.
    fn is_variable_len(&self) -> bool {
        matches!(self, Self::TokenMetadata(_))
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
            // Handled by the variable-length path; never reaches `init`.
            Self::TokenMetadata(_) => {
                unreachable!("TokenMetadata is initialized via the variable-length path")
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
#[non_exhaustive]
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

// Duplicates would silently overwrite each other in the TLV data.
fn assert_unique_extension_types(extension_types: &[ExtensionType]) {
    for (index, extension_type) in extension_types.iter().enumerate() {
        assert!(
            !extension_types[..index].contains(extension_type),
            "duplicate extension: {extension_type:?}"
        );
    }
}

/// Create a Mint Account with extensions
///
/// # Example
///
/// Build a mint that routes transfers through a transfer-hook program, then a
/// token account carrying the extensions that mint requires:
///
/// ```
/// use mollusk_svm_programs_token_2022::{
///     create_account_for_mint_with_extensions,
///     create_account_for_token_account_with_extensions,
///     required_account_extensions_for_mint, AccountState, Mint, MintExtension,
///     TokenAccount, TransferHook,
/// };
/// use {solana_program_option::COption, solana_pubkey::Pubkey};
///
/// let authority = Pubkey::new_unique();
/// let hook_program = Pubkey::new_unique();
///
/// let mint_account = create_account_for_mint_with_extensions(
///     Mint {
///         mint_authority: COption::Some(authority),
///         supply: 0,
///         decimals: 9,
///         is_initialized: true,
///         freeze_authority: COption::None,
///     },
///     &[MintExtension::TransferHook(TransferHook {
///         authority: Some(authority).try_into().unwrap(),
///         program_id: Some(hook_program).try_into().unwrap(),
///     })],
/// );
///
/// // A transfer-hook mint requires its token accounts to carry
/// // `TransferHookAccount`; derive that set straight from the mint.
/// let token_account = create_account_for_token_account_with_extensions(
///     TokenAccount {
///         mint: Pubkey::new_unique(),
///         owner: Pubkey::new_unique(),
///         amount: 0,
///         delegate: COption::None,
///         state: AccountState::Initialized,
///         is_native: COption::None,
///         delegated_amount: 0,
///         close_authority: COption::None,
///     },
///     &required_account_extensions_for_mint(&mint_account.data),
/// );
/// ```
pub fn create_account_for_mint_with_extensions(
    mint_data: Mint,
    extensions: &[MintExtension],
) -> Account {
    assert_unique_extension_types(
        &extensions
            .iter()
            .map(MintExtension::extension_type)
            .collect::<Vec<_>>(),
    );

    let (variable_len_extensions, fixed_len_extensions): (Vec<_>, Vec<_>) = extensions
        .iter()
        .partition(|extension| extension.is_variable_len());

    let fixed_len_types: Vec<ExtensionType> = fixed_len_extensions
        .iter()
        .map(|extension| extension.extension_type())
        .collect();

    let len = ExtensionType::try_calculate_account_len::<Mint>(&fixed_len_types).unwrap();
    let mut data = vec![0u8; len];

    {
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();
        for extension in fixed_len_extensions {
            extension.init(&mut state);
        }
    }

    // variable-length extensions require growing the buffer first
    for extension in variable_len_extensions {
        match extension {
            MintExtension::TokenMetadata(metadata) => {
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
            _ => unreachable!("unhandled variable-length extension: {extension:?}"),
        }
    }

    let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();
    state.base = mint_data;
    state.pack_base();
    state.init_account_type().unwrap();

    rent_exempt_account(data)
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
    assert_unique_extension_types(&extension_types);

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

    rent_exempt_account(data)
}

/// Get the account extensions required by a mint's extensions
///
/// Panics if the mint account data is invalid, or if a required extension
/// has no [`TokenAccountExtension`] counterpart
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
            ExtensionType::ImmutableOwner => TokenAccountExtension::ImmutableOwner,
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
        crate::{add_program, ID},
        mollusk_svm::{result::Check, Mollusk},
        solana_account::Account,
        solana_program_option::COption,
        solana_pubkey::Pubkey,
        spl_token_2022_interface::{
            extension::transfer_fee::TransferFee,
            instruction::{mint_to, transfer_checked},
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

    #[test]
    fn transfer_checked_withholds_fee() {
        let mut mollusk = Mollusk::default();
        add_program(&mut mollusk);

        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let mint_account = create_account_for_mint_with_extensions(
            Mint {
                supply: 10_000,
                ..mint_data(authority)
            },
            &[MintExtension::TransferFeeConfig(transfer_fee_config(
                authority,
            ))],
        );
        let required = required_account_extensions_for_mint(&mint_account.data);
        let source_account = create_account_for_token_account_with_extensions(
            token_account_data(mint, owner, 10_000),
            &required,
        );
        let destination_account = create_account_for_token_account_with_extensions(
            token_account_data(mint, owner, 0),
            &required,
        );

        let instruction =
            transfer_checked(&ID, &source, &mint, &destination, &owner, &[], 10_000, 9).unwrap();
        let result = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (source, source_account),
                (mint, mint_account),
                (destination, destination_account),
                (owner, Account::default()),
            ],
            &[Check::success()],
        );

        // 100 basis points of 10_000 are withheld on the destination
        let resulting_destination = result.get_account(&destination).unwrap();
        let state =
            StateWithExtensions::<TokenAccount>::unpack(&resulting_destination.data).unwrap();
        assert_eq!(state.base.amount, 9_900);
        assert_eq!(
            state
                .get_extension::<TransferFeeAmount>()
                .unwrap()
                .withheld_amount,
            100.into()
        );

        let resulting_source = result.get_account(&source).unwrap();
        let state = StateWithExtensions::<TokenAccount>::unpack(&resulting_source.data).unwrap();
        assert_eq!(state.base.amount, 0);
    }

    #[test]
    fn required_extensions_for_non_transferable_mint() {
        let mint_account = create_account_for_mint_with_extensions(
            mint_data(Pubkey::new_unique()),
            &[MintExtension::NonTransferable],
        );
        let required = required_account_extensions_for_mint(&mint_account.data);
        assert!(required.contains(&TokenAccountExtension::NonTransferableAccount));
        assert!(required.contains(&TokenAccountExtension::ImmutableOwner));

        let token_account = create_account_for_token_account_with_extensions(
            token_account_data(Pubkey::new_unique(), Pubkey::new_unique(), 0),
            &required,
        );
        let state = StateWithExtensions::<TokenAccount>::unpack(&token_account.data).unwrap();
        state.get_extension::<NonTransferableAccount>().unwrap();
        state.get_extension::<ImmutableOwner>().unwrap();
    }

    #[test]
    fn all_extension_variants_round_trip() {
        let metadata = TokenMetadata {
            update_authority: None.try_into().unwrap(),
            mint: Pubkey::new_unique(),
            name: "Mollusk".to_string(),
            symbol: "MLSK".to_string(),
            uri: "https://example.com".to_string(),
            additional_metadata: vec![],
        };
        let mint_account = create_account_for_mint_with_extensions(
            mint_data(Pubkey::new_unique()),
            &[
                MintExtension::TransferFeeConfig(TransferFeeConfig::default()),
                MintExtension::MintCloseAuthority(MintCloseAuthority::default()),
                MintExtension::DefaultAccountState(DefaultAccountState::default()),
                MintExtension::NonTransferable,
                MintExtension::InterestBearingConfig(InterestBearingConfig::default()),
                MintExtension::PermanentDelegate(PermanentDelegate::default()),
                MintExtension::TransferHook(TransferHook::default()),
                MintExtension::MetadataPointer(MetadataPointer::default()),
                MintExtension::TokenMetadata(metadata.clone()),
                MintExtension::GroupPointer(GroupPointer::default()),
                MintExtension::TokenGroup(TokenGroup::default()),
                MintExtension::GroupMemberPointer(GroupMemberPointer::default()),
                MintExtension::TokenGroupMember(TokenGroupMember::default()),
                MintExtension::ScaledUiAmount(ScaledUiAmountConfig::default()),
                MintExtension::Pausable(PausableConfig::default()),
                MintExtension::PermissionedBurn(PermissionedBurnConfig::default()),
            ],
        );
        let state = StateWithExtensions::<Mint>::unpack(&mint_account.data).unwrap();
        assert_eq!(state.get_extension_types().unwrap().len(), 16);
        assert_eq!(
            state.get_variable_len_extension::<TokenMetadata>().unwrap(),
            metadata
        );

        let token_account = create_account_for_token_account_with_extensions(
            token_account_data(Pubkey::new_unique(), Pubkey::new_unique(), 0),
            &[
                TokenAccountExtension::TransferFeeAmount(TransferFeeAmount::default()),
                TokenAccountExtension::ImmutableOwner,
                TokenAccountExtension::MemoTransfer(MemoTransfer::default()),
                TokenAccountExtension::CpiGuard(CpiGuard::default()),
                TokenAccountExtension::NonTransferableAccount,
                TokenAccountExtension::TransferHookAccount(TransferHookAccount::default()),
                TokenAccountExtension::PausableAccount,
            ],
        );
        let state = StateWithExtensions::<TokenAccount>::unpack(&token_account.data).unwrap();
        assert_eq!(state.get_extension_types().unwrap().len(), 7);
    }

    #[test]
    #[should_panic(expected = "duplicate extension")]
    fn duplicate_extension_panics() {
        create_account_for_mint_with_extensions(
            mint_data(Pubkey::new_unique()),
            &[
                MintExtension::NonTransferable,
                MintExtension::NonTransferable,
            ],
        );
    }
}
