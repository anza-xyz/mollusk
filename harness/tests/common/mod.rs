//! Shared helpers for integration tests.

#![allow(dead_code)]

use solana_svm_feature_set::SVMFeatureSet;

/// Field-by-field equality check for `SVMFeatureSet`, since the upstream
/// type does not derive `PartialEq`.
pub fn compare_svm_feature_sets(a: &SVMFeatureSet, b: &SVMFeatureSet) {
    macro_rules! cmp {
        ($($field:ident),* $(,)?) => {
            $(
                assert_eq!(
                    a.$field,
                    b.$field,
                    "SVMFeatureSet field `{}` differs",
                    stringify!($field),
                );
            )*
        };
    }
    cmp! {
        move_precompile_verification_to_svm,
        syscall_parameter_address_restrictions,
        virtual_address_space_adjustments,
        account_data_direct_mapping,
        enable_bpf_loader_set_authority_checked_ix,
        enable_loader_v4,
        deplete_cu_meter_on_vm_failure,
        abort_on_invalid_curve,
        blake3_syscall_enabled,
        curve25519_syscall_enabled,
        disable_deploy_of_alloc_free_syscall,
        disable_fees_sysvar,
        disable_sbpf_v0_execution,
        enable_alt_bn128_compression_syscall,
        enable_alt_bn128_syscall,
        enable_big_mod_exp_syscall,
        enable_get_epoch_stake_syscall,
        enable_poseidon_syscall,
        enable_sbpf_v1_deployment_and_execution,
        enable_sbpf_v2_deployment_and_execution,
        enable_sbpf_v3_deployment_and_execution,
        get_sysvar_syscall_enabled,
        last_restart_slot_sysvar,
        reenable_sbpf_v0_execution,
        remaining_compute_units_syscall_enabled,
        remove_bpf_loader_incorrect_program_id,
        move_stake_and_move_lamports_ixs,
        deprecate_legacy_vote_ixs,
        simplify_alt_bn128_syscall_error_codes,
        fix_alt_bn128_multiplication_input_length,
        increase_tx_account_lock_limit,
        enable_extend_program_checked,
        formalize_loaded_transaction_data_size,
        disable_zk_elgamal_proof_program,
        reenable_zk_elgamal_proof_program,
        delay_commission_updates,
        raise_cpi_nesting_limit_to_8,
        provide_instruction_data_offset_in_vm_r2,
        increase_cpi_account_info_limit,
        vote_state_v4,
        poseidon_enforce_padding,
        fix_alt_bn128_pairing_length_check,
        alt_bn128_little_endian,
        create_account_allow_prefund,
        bls_pubkey_management_in_vote_account,
        enable_alt_bn128_g2_syscalls,
        commission_rate_in_basis_points,
        custom_commission_collector,
        enable_bls12_381_syscall,
        block_revenue_sharing,
        vote_account_initialize_v2,
        direct_account_pointers_in_program_input,
    }
}
