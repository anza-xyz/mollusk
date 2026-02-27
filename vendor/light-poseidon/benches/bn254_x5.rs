use ark_bn254::Fr;
use ark_ff::PrimeField;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::Rng;

use light_poseidon::{Poseidon, PoseidonHasher};

pub fn bench_poseidon_bn254_x5(c: &mut Criterion) {
    let mut inputs = Vec::new();
    for i in 1..13 {
        let name = [String::from("poseidon_bn254_x5_"), i.to_string()].concat();
        let random_bytes1 = Fr::from_be_bytes_mod_order(&rand::thread_rng().gen::<[u8; 32]>());
        inputs.push(random_bytes1);
        let mut hasher = Poseidon::<Fr>::new_circom(i).unwrap();

        c.bench_function(&name, |b| b.iter(|| hasher.hash(&inputs[..])));
    }
}

criterion_group!(benches, bench_poseidon_bn254_x5);
criterion_main!(benches);
