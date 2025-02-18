//! Compute budget for instructions.

use {
    super::proto::ComputeBudget as ProtoComputeBudget,
    solana_compute_budget::compute_budget::ComputeBudget, solana_keccak_hasher::Hasher,
};

impl From<ProtoComputeBudget> for ComputeBudget {
    fn from(value: ProtoComputeBudget) -> Self {
        let ProtoComputeBudget {
            compute_unit_limit,
            log_64_units,
            create_program_address_units,
            invoke_units,
            max_instruction_stack_depth,
            max_instruction_trace_length,
            sha256_base_cost,
            sha256_byte_cost,
            sha256_max_slices,
            max_call_depth,
            stack_frame_size,
            log_pubkey_units,
            max_cpi_instruction_size,
            cpi_bytes_per_unit,
            sysvar_base_cost,
            secp256k1_recover_cost,
            syscall_base_cost,
            curve25519_edwards_validate_point_cost,
            curve25519_edwards_add_cost,
            curve25519_edwards_subtract_cost,
            curve25519_edwards_multiply_cost,
            curve25519_edwards_msm_base_cost,
            curve25519_edwards_msm_incremental_cost,
            curve25519_ristretto_validate_point_cost,
            curve25519_ristretto_add_cost,
            curve25519_ristretto_subtract_cost,
            curve25519_ristretto_multiply_cost,
            curve25519_ristretto_msm_base_cost,
            curve25519_ristretto_msm_incremental_cost,
            heap_size,
            heap_cost,
            mem_op_base_cost,
            alt_bn128_addition_cost,
            alt_bn128_multiplication_cost,
            alt_bn128_pairing_one_pair_cost_first,
            alt_bn128_pairing_one_pair_cost_other,
            big_modular_exponentiation_base_cost,
            big_modular_exponentiation_cost_divisor,
            poseidon_cost_coefficient_a,
            poseidon_cost_coefficient_c,
            get_remaining_compute_units_cost,
            alt_bn128_g1_compress,
            alt_bn128_g1_decompress,
            alt_bn128_g2_compress,
            alt_bn128_g2_decompress,
        } = value;

        Self {
            compute_unit_limit,
            log_64_units,
            create_program_address_units,
            invoke_units,
            max_instruction_stack_depth: max_instruction_stack_depth as usize,
            max_instruction_trace_length: max_instruction_trace_length as usize,
            sha256_base_cost,
            sha256_byte_cost,
            sha256_max_slices,
            max_call_depth: max_call_depth as usize,
            stack_frame_size: stack_frame_size as usize,
            log_pubkey_units,
            max_cpi_instruction_size: max_cpi_instruction_size as usize,
            cpi_bytes_per_unit,
            sysvar_base_cost,
            secp256k1_recover_cost,
            syscall_base_cost,
            curve25519_edwards_validate_point_cost,
            curve25519_edwards_add_cost,
            curve25519_edwards_subtract_cost,
            curve25519_edwards_multiply_cost,
            curve25519_edwards_msm_base_cost,
            curve25519_edwards_msm_incremental_cost,
            curve25519_ristretto_validate_point_cost,
            curve25519_ristretto_add_cost,
            curve25519_ristretto_subtract_cost,
            curve25519_ristretto_multiply_cost,
            curve25519_ristretto_msm_base_cost,
            curve25519_ristretto_msm_incremental_cost,
            heap_size,
            heap_cost,
            mem_op_base_cost,
            alt_bn128_addition_cost,
            alt_bn128_multiplication_cost,
            alt_bn128_pairing_one_pair_cost_first,
            alt_bn128_pairing_one_pair_cost_other,
            big_modular_exponentiation_base_cost,
            big_modular_exponentiation_cost_divisor,
            poseidon_cost_coefficient_a,
            poseidon_cost_coefficient_c,
            get_remaining_compute_units_cost,
            alt_bn128_g1_compress,
            alt_bn128_g1_decompress,
            alt_bn128_g2_compress,
            alt_bn128_g2_decompress,
        }
    }
}

impl From<ComputeBudget> for ProtoComputeBudget {
    fn from(value: ComputeBudget) -> Self {
        let ComputeBudget {
            compute_unit_limit,
            log_64_units,
            create_program_address_units,
            invoke_units,
            max_instruction_stack_depth,
            max_instruction_trace_length,
            sha256_base_cost,
            sha256_byte_cost,
            sha256_max_slices,
            max_call_depth,
            stack_frame_size,
            log_pubkey_units,
            max_cpi_instruction_size,
            cpi_bytes_per_unit,
            sysvar_base_cost,
            secp256k1_recover_cost,
            syscall_base_cost,
            curve25519_edwards_validate_point_cost,
            curve25519_edwards_add_cost,
            curve25519_edwards_subtract_cost,
            curve25519_edwards_multiply_cost,
            curve25519_edwards_msm_base_cost,
            curve25519_edwards_msm_incremental_cost,
            curve25519_ristretto_validate_point_cost,
            curve25519_ristretto_add_cost,
            curve25519_ristretto_subtract_cost,
            curve25519_ristretto_multiply_cost,
            curve25519_ristretto_msm_base_cost,
            curve25519_ristretto_msm_incremental_cost,
            heap_size,
            heap_cost,
            mem_op_base_cost,
            alt_bn128_addition_cost,
            alt_bn128_multiplication_cost,
            alt_bn128_pairing_one_pair_cost_first,
            alt_bn128_pairing_one_pair_cost_other,
            big_modular_exponentiation_base_cost,
            big_modular_exponentiation_cost_divisor,
            poseidon_cost_coefficient_a,
            poseidon_cost_coefficient_c,
            get_remaining_compute_units_cost,
            alt_bn128_g1_compress,
            alt_bn128_g1_decompress,
            alt_bn128_g2_compress,
            alt_bn128_g2_decompress,
        } = value;

        Self {
            compute_unit_limit,
            log_64_units,
            create_program_address_units,
            invoke_units,
            max_instruction_stack_depth: max_instruction_stack_depth as u64,
            max_instruction_trace_length: max_instruction_trace_length as u64,
            sha256_base_cost,
            sha256_byte_cost,
            sha256_max_slices,
            max_call_depth: max_call_depth as u64,
            stack_frame_size: stack_frame_size as u64,
            log_pubkey_units,
            max_cpi_instruction_size: max_cpi_instruction_size as u64,
            cpi_bytes_per_unit,
            sysvar_base_cost,
            secp256k1_recover_cost,
            syscall_base_cost,
            curve25519_edwards_validate_point_cost,
            curve25519_edwards_add_cost,
            curve25519_edwards_subtract_cost,
            curve25519_edwards_multiply_cost,
            curve25519_edwards_msm_base_cost,
            curve25519_edwards_msm_incremental_cost,
            curve25519_ristretto_validate_point_cost,
            curve25519_ristretto_add_cost,
            curve25519_ristretto_subtract_cost,
            curve25519_ristretto_multiply_cost,
            curve25519_ristretto_msm_base_cost,
            curve25519_ristretto_msm_incremental_cost,
            heap_size,
            heap_cost,
            mem_op_base_cost,
            alt_bn128_addition_cost,
            alt_bn128_multiplication_cost,
            alt_bn128_pairing_one_pair_cost_first,
            alt_bn128_pairing_one_pair_cost_other,
            big_modular_exponentiation_base_cost,
            big_modular_exponentiation_cost_divisor,
            poseidon_cost_coefficient_a,
            poseidon_cost_coefficient_c,
            get_remaining_compute_units_cost,
            alt_bn128_g1_compress,
            alt_bn128_g1_decompress,
            alt_bn128_g2_compress,
            alt_bn128_g2_decompress,
        }
    }
}

pub(crate) fn hash_proto_compute_budget(hasher: &mut Hasher, compute_budget: &ProtoComputeBudget) {
    hasher.hash(&compute_budget.compute_unit_limit.to_le_bytes());
    hasher.hash(&compute_budget.log_64_units.to_le_bytes());
    hasher.hash(&compute_budget.create_program_address_units.to_le_bytes());
    hasher.hash(&compute_budget.invoke_units.to_le_bytes());
    hasher.hash(&compute_budget.max_instruction_stack_depth.to_le_bytes());
    hasher.hash(&compute_budget.max_instruction_trace_length.to_le_bytes());
    hasher.hash(&compute_budget.sha256_base_cost.to_le_bytes());
    hasher.hash(&compute_budget.sha256_byte_cost.to_le_bytes());
    hasher.hash(&compute_budget.sha256_max_slices.to_le_bytes());
    hasher.hash(&compute_budget.max_call_depth.to_le_bytes());
    hasher.hash(&compute_budget.stack_frame_size.to_le_bytes());
    hasher.hash(&compute_budget.log_pubkey_units.to_le_bytes());
    hasher.hash(&compute_budget.max_cpi_instruction_size.to_le_bytes());
    hasher.hash(&compute_budget.cpi_bytes_per_unit.to_le_bytes());
    hasher.hash(&compute_budget.sysvar_base_cost.to_le_bytes());
    hasher.hash(&compute_budget.secp256k1_recover_cost.to_le_bytes());
    hasher.hash(&compute_budget.syscall_base_cost.to_le_bytes());
    hasher.hash(
        &compute_budget
            .curve25519_edwards_validate_point_cost
            .to_le_bytes(),
    );
    hasher.hash(&compute_budget.curve25519_edwards_add_cost.to_le_bytes());
    hasher.hash(
        &compute_budget
            .curve25519_edwards_subtract_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_edwards_multiply_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_edwards_msm_base_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_edwards_msm_incremental_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_ristretto_validate_point_cost
            .to_le_bytes(),
    );
    hasher.hash(&compute_budget.curve25519_ristretto_add_cost.to_le_bytes());
    hasher.hash(
        &compute_budget
            .curve25519_ristretto_subtract_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_ristretto_multiply_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_ristretto_msm_base_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .curve25519_ristretto_msm_incremental_cost
            .to_le_bytes(),
    );
    hasher.hash(&compute_budget.heap_size.to_le_bytes());
    hasher.hash(&compute_budget.heap_cost.to_le_bytes());
    hasher.hash(&compute_budget.mem_op_base_cost.to_le_bytes());
    hasher.hash(&compute_budget.alt_bn128_addition_cost.to_le_bytes());
    hasher.hash(&compute_budget.alt_bn128_multiplication_cost.to_le_bytes());
    hasher.hash(
        &compute_budget
            .alt_bn128_pairing_one_pair_cost_first
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .alt_bn128_pairing_one_pair_cost_other
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .big_modular_exponentiation_base_cost
            .to_le_bytes(),
    );
    hasher.hash(
        &compute_budget
            .big_modular_exponentiation_cost_divisor
            .to_le_bytes(),
    );
    hasher.hash(&compute_budget.poseidon_cost_coefficient_a.to_le_bytes());
    hasher.hash(&compute_budget.poseidon_cost_coefficient_c.to_le_bytes());
    hasher.hash(
        &compute_budget
            .get_remaining_compute_units_cost
            .to_le_bytes(),
    );
    hasher.hash(&compute_budget.alt_bn128_g1_compress.to_le_bytes());
    hasher.hash(&compute_budget.alt_bn128_g1_decompress.to_le_bytes());
    hasher.hash(&compute_budget.alt_bn128_g2_compress.to_le_bytes());
    hasher.hash(&compute_budget.alt_bn128_g2_decompress.to_le_bytes());
}
