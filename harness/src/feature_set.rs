//! Conversion between the SVM-level feature set and the full Agave feature set.
//!
//! Required for fixtures, precompiles, and other APIs that may come from Agave
//! libraries higher than or external to SVM.

use {agave_feature_set::FeatureSet, solana_svm_feature_set::SVMFeatureSet};

pub fn svm_feature_set_to_feature_set(svm: &SVMFeatureSet) -> FeatureSet {
    let mut fs = FeatureSet::all_enabled();
    macro_rules! gate {
        ($($field:ident),* $(,)?) => {
            $(
                if !svm.$field {
                    fs.deactivate(&agave_feature_set::$field::id());
                }
            )*
        };
    }
    gate! {
        move_precompile_verification_to_svm,
        syscall_parameter_address_restrictions,
        virtual_address_space_adjustments,
        account_data_direct_mapping,
        enable_bpf_loader_set_authority_checked_ix,
        deplete_cu_meter_on_vm_failure,
        abort_on_invalid_curve,
        blake3_syscall_enabled,
        curve25519_syscall_enabled,
        disable_fees_sysvar,
        disable_sbpf_v0_execution,
        disable_sbpf_v0_v1_v2_deployment,
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
        formalize_loaded_transaction_data_size,
        disable_zk_elgamal_proof_program,
        reenable_zk_elgamal_proof_program,
        delay_commission_updates,
        raise_cpi_nesting_limit_to_8,
        increase_cpi_account_info_limit,
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
        loader_v3_minimum_extend_program_size,
        enable_sha512_syscall,
        relax_post_exec_min_balance_check,
        define_ltds_fee_only_semantics,
        program_runtime_abiv2,
    }
    fs
}
