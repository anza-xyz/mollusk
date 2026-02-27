use {
    criterion::{black_box, criterion_group, criterion_main, Criterion},
    serde::Deserialize,
    solana_big_mod_exp::big_mod_exp,
};

#[derive(Deserialize, Debug)]
struct BenchmarkData {
    exp_3_and_65537: BitSizeGroup,
    variable_exponents: VariableBitSizeGroup,
}

#[derive(Deserialize, Debug)]
struct BitSizeGroup {
    bits_512: BaseModSet,
    bits_1024: BaseModSet,
    bits_2048: BaseModSet,
    bits_4096: BaseModSet,
}

#[derive(Deserialize, Debug)]
struct BaseModSet {
    base: Vec<u8>,
    modulus_odd: Vec<u8>,
    modulus_even: Vec<u8>,
}

#[derive(Deserialize, Debug)]
struct VariableBitSizeGroup {
    bits_512: VariableSet,
    bits_1024: VariableSet,
    bits_2048: VariableSet,
    bits_4096: VariableSet,
}

#[derive(Deserialize, Debug)]
struct VariableSet {
    base: Vec<u8>,
    modulus: Vec<u8>,
    exponent: Vec<u8>,
}

fn all_benches(c: &mut Criterion) {
    let data_str = include_str!("data/benchmark_constants.json");
    let data: BenchmarkData =
        serde_json::from_str(data_str).expect("Failed to parse benchmark data");

    // --- Benchmark Group for Exponent 3 ---
    let mut group_exp_3 = c.benchmark_group("Exponent 3");
    let exponent_3 = [3u8];
    let const_exp_data = &data.exp_3_and_65537;

    group_exp_3.bench_function("512 bits odd", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &const_exp_data.bits_512.base,
                &exponent_3,
                &const_exp_data.bits_512.modulus_odd,
            ))
        })
    });
    group_exp_3.bench_function("512 bits even", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &const_exp_data.bits_512.base,
                &exponent_3,
                &const_exp_data.bits_512.modulus_even,
            ))
        })
    });
    group_exp_3.bench_function("1024 bits odd", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &const_exp_data.bits_1024.base,
                &exponent_3,
                &const_exp_data.bits_1024.modulus_odd,
            ))
        })
    });
    group_exp_3.bench_function("1024 bits even", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &const_exp_data.bits_1024.base,
                &exponent_3,
                &const_exp_data.bits_1024.modulus_even,
            ))
        })
    });
    group_exp_3.finish();

    // --- Benchmark Group for Exponent 65537 ---
    let mut group_exp_65537 = c.benchmark_group("Exponent 65537");
    let exponent_65537 = [1, 0, 1];

    group_exp_65537.bench_function("2048 bits odd", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &const_exp_data.bits_2048.base,
                &exponent_65537,
                &const_exp_data.bits_2048.modulus_odd,
            ))
        })
    });
    group_exp_65537.bench_function("4096 bits odd", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &const_exp_data.bits_4096.base,
                &exponent_65537,
                &const_exp_data.bits_4096.modulus_odd,
            ))
        })
    });
    group_exp_65537.finish();

    // --- Benchmark Group for Variable Exponents ---
    let mut group_variable = c.benchmark_group("Variable Exponents");
    let var_exp_data = &data.variable_exponents;

    group_variable.bench_function("512-bit exp, 512-bit mod", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &var_exp_data.bits_512.base,
                &var_exp_data.bits_512.exponent,
                &var_exp_data.bits_512.modulus,
            ))
        })
    });
    group_variable.bench_function("1024-bit exp, 1024-bit mod", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &var_exp_data.bits_1024.base,
                &var_exp_data.bits_1024.exponent,
                &var_exp_data.bits_1024.modulus,
            ))
        })
    });
    group_variable.bench_function("2048-bit exp, 2048-bit mod", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &var_exp_data.bits_2048.base,
                &var_exp_data.bits_2048.exponent,
                &var_exp_data.bits_2048.modulus,
            ))
        })
    });
    group_variable.bench_function("4096-bit exp, 4096-bit mod", |b| {
        b.iter(|| {
            black_box(big_mod_exp(
                &var_exp_data.bits_4096.base,
                &var_exp_data.bits_4096.exponent,
                &var_exp_data.bits_4096.modulus,
            ))
        })
    });
    group_variable.finish();
}

criterion_group!(benches, all_benches);
criterion_main!(benches);
