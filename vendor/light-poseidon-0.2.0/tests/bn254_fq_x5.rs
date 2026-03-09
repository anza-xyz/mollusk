use ark_bn254::Fr;
use ark_ff::{BigInteger, BigInteger256, One, PrimeField, UniformRand, Zero};
use light_poseidon::{
    bytes_to_prime_field_element_be, bytes_to_prime_field_element_le, validate_bytes_length,
    Poseidon, PoseidonError,
};
use light_poseidon::{PoseidonBytesHasher, PoseidonHasher};
use rand::Rng;

/// Checks the hash of `1` as a prime field element.
#[test]
fn test_poseidon_one() {
    let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();

    let expected = [
        0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167, 138,
        203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129,
    ];

    let input = Fr::from_be_bytes_mod_order(&[1u8]);
    let hash = hasher.hash(&[input, input]).unwrap();

    assert_eq!(hash.into_bigint().to_bytes_be(), expected,);

    let input = Fr::from_be_bytes_mod_order(&[0u8, 1u8]);
    let hash = hasher.hash(&[input, input]).unwrap();

    assert_eq!(hash.into_bigint().to_bytes_be(), expected);

    let input = Fr::from_be_bytes_mod_order(&[0u8, 0u8, 1u8]);
    let hash = hasher.hash(&[input, input]).unwrap();

    assert_eq!(hash.into_bigint().to_bytes_be(), expected);
}

/// Checks the hash of byte slices consistng of ones and twos.
#[test]
fn test_poseidon_bn254_x5_fq_input_ones_twos() {
    let input1 = Fr::from_be_bytes_mod_order(&[1u8; 32]);
    let input2 = Fr::from_be_bytes_mod_order(&[2u8; 32]);
    let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
    let hash = hasher.hash(&[input1, input2]).unwrap();
    assert_eq!(
        hash.into_bigint().to_bytes_be(),
        [
            13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123, 132,
            254, 156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144
        ]
    );

    let hash = hasher.hash_bytes_be(&[&[1u8; 32], &[2u8; 32]]).unwrap();
    assert_eq!(
        hash,
        [
            13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123, 132,
            254, 156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144
        ]
    );

    let hash = hasher.hash_bytes_le(&[&[1u8; 32], &[2u8; 32]]).unwrap();
    assert_eq!(
        hash,
        [
            144, 25, 130, 41, 200, 53, 231, 38, 27, 206, 162, 156, 254, 132, 123, 32, 25, 99, 242,
            85, 3, 94, 235, 125, 28, 140, 138, 143, 147, 225, 84, 13
        ]
    )
}

/// Checks thebash of bytes slices consisting of ones and twos, with a custom
/// domain tag.
#[test]
fn test_poseidon_bn254_x5_fq_with_domain_tag() {
    let input1 = Fr::from_be_bytes_mod_order(&[1u8; 32]);
    let input2 = Fr::from_be_bytes_mod_order(&[2u8; 32]);
    let mut hasher = Poseidon::<Fr>::with_domain_tag_circom(2, Fr::zero()).unwrap();
    let hash = hasher.hash(&[input1, input2]).unwrap();

    let expected_tag_zero = [
        13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123, 132, 254,
        156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144,
    ];

    assert_eq!(hash.into_bigint().to_bytes_be(), expected_tag_zero);

    let mut hasher = Poseidon::<Fr>::with_domain_tag_circom(2, Fr::one()).unwrap();
    let hash = hasher.hash(&[input1, input2]).unwrap();

    assert_ne!(hash.into_bigint().to_bytes_be(), expected_tag_zero);
}

/// Checks the hash of one and two.
#[test]
fn test_poseidon_bn254_x5_fq_input_one_two() {
    let input1 = Fr::from_be_bytes_mod_order(&[1]);
    let input2 = Fr::from_be_bytes_mod_order(&[2]);

    let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
    let hash = hasher.hash(&[input1, input2]).unwrap();

    assert_eq!(
        hash.into_bigint().to_bytes_le(),
        [
            154, 24, 23, 68, 122, 96, 25, 158, 81, 69, 50, 116, 242, 23, 54, 42, 207, 233, 98, 150,
            107, 76, 246, 61, 65, 144, 214, 231, 245, 192, 92, 17
        ]
    );
}

#[test]
fn test_poseidon_bn254_x5_fq_input_random() {
    let input1 = Fr::from_be_bytes_mod_order(&[
        0x06, 0x9c, 0x63, 0x81, 0xac, 0x0b, 0x96, 0x8e, 0x88, 0x1c, 0x91, 0x3c, 0x17, 0xd8, 0x36,
        0x06, 0x7f, 0xd1, 0x5f, 0x2c, 0xc7, 0x9f, 0x90, 0x2c, 0x80, 0x70, 0xb3, 0x6d, 0x28, 0x66,
        0x17, 0xdd,
    ]);
    let input2 = Fr::from_be_bytes_mod_order(&[
        0xc3, 0x3b, 0x60, 0x04, 0x2f, 0x76, 0xc7, 0xfb, 0xd0, 0x5d, 0xb7, 0x76, 0x23, 0xcb, 0x17,
        0xb8, 0x1d, 0x49, 0x41, 0x4b, 0x82, 0xe5, 0x6a, 0x2e, 0xc0, 0x18, 0xf7, 0xa5, 0x5c, 0x3f,
        0x30, 0x0b,
    ]);

    let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
    let hash = hasher.hash(&[input1, input2]).unwrap();
    assert_eq!(
        hash.into_bigint().to_bytes_le(),
        [
            75, 85, 249, 42, 66, 238, 230, 151, 158, 90, 250, 51, 131, 212, 131, 18, 151, 235, 96,
            103, 135, 243, 186, 61, 173, 135, 52, 77, 132, 173, 19, 10
        ]
    )
}

/// Check whther providing different number of inputs than supported by the
/// hasher results in an error.
#[test]
fn test_poseidon_bn254_x5_fq_too_many_inputs() {
    let mut rng = rand::thread_rng();

    for i in 1..13 {
        let mut hasher = Poseidon::<Fr>::new_circom(i).unwrap();

        for j in 1..13 {
            if i != j {
                let inputs: Vec<_> = (0..j).map(|_| Fr::rand(&mut rng)).collect();
                let res = hasher.hash(&inputs);
                assert!(res.is_err());

                let inputs_bytes_be: Vec<_> = inputs
                    .iter()
                    .map(|i| i.into_bigint().to_bytes_be())
                    .collect();
                let inputs_bytes_be: Vec<&[u8]> = inputs_bytes_be.iter().map(|v| &v[..]).collect();
                let res_bytes_be = hasher.hash_bytes_be(&inputs_bytes_be);
                assert!(res_bytes_be.is_err());

                let inputs_bytes_le: Vec<_> = inputs
                    .iter()
                    .map(|i| i.into_bigint().to_bytes_le())
                    .collect();
                let inputs_bytes_le: Vec<&[u8]> = inputs_bytes_le.iter().map(|v| &v[..]).collect();
                let res_bytes_le = hasher.hash_bytes_le(&inputs_bytes_le);
                assert!(res_bytes_le.is_err());
            }
        }
    }
}

/// Check whether byte inputs with length lower than the byte limit indicated
/// by the modulus produce the same hashes as equivalent byte inputs padded with
/// zeros. They should be serialized as the same prime field elements.
#[test]
fn test_poseidon_bn254_x5_fq_smaller_arrays() {
    let mut hasher = Poseidon::<Fr>::new_circom(1).unwrap();

    let input1 = vec![1; 1];
    let hash1 = hasher.hash_bytes_le(&[input1.as_slice()]).unwrap();

    for len in 2..32 {
        let input = [vec![1u8], vec![0; len - 1]].concat();
        let hash = hasher.hash_bytes_le(&[input.as_slice()]).unwrap();

        assert_eq!(hash, hash1);
    }

    let input1 = vec![1; 1];
    let hash1 = hasher.hash_bytes_be(&[input1.as_slice()]).unwrap();

    for len in 2..32 {
        let input = [vec![0; len - 1], vec![1u8]].concat();
        let hash = hasher.hash_bytes_be(&[input.as_slice()]).unwrap();

        assert_eq!(hash, hash1);
    }
}

/// Check whether big-endian byte inputs with length lower than the byte limit
/// indicated by the modulus produce the same hashes as equivalent byte inputs
/// padded with zeros. Randomize the byte slices and try all the possible
/// lengths. They should be serialized as the same prime field elements.
#[test]
fn test_poseidon_bn254_x5_fq_hash_bytes_be_smaller_arrays_random() {
    for nr_inputs in 1..12 {
        let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();
        for smaller_arr_len in 1..31 {
            let inputs: Vec<Vec<u8>> = (0..nr_inputs)
                .map(|_| {
                    let rng = rand::thread_rng();
                    rng.sample_iter(rand::distributions::Standard)
                        .take(smaller_arr_len)
                        .collect()
                })
                .collect();
            let inputs: Vec<&[u8]> = inputs.iter().map(|v| &v[..]).collect();
            let hash1 = hasher.hash_bytes_be(inputs.as_slice()).unwrap();

            for greater_arr_len in smaller_arr_len + 1..32 {
                let inputs: Vec<Vec<u8>> = inputs
                    .iter()
                    .map(|input| {
                        [vec![0u8; greater_arr_len - smaller_arr_len], input.to_vec()].concat()
                    })
                    .collect();
                let inputs: Vec<&[u8]> = inputs.iter().map(|v| &v[..]).collect();
                let hash = hasher.hash_bytes_be(inputs.as_slice()).unwrap();

                assert_eq!(
                    hash, hash1,
                    "inputs: {nr_inputs}, smaller array length: {smaller_arr_len}, greater array length: {greater_arr_len}"
                );
            }
        }
    }
}

/// Check whether little-endian byte inputs with length lower than the byte limit
/// indicated by the modulus produce the same hashes as equivalent byte inputs
/// padded with zeros. Randomize the byte slices and try all the possible
/// lengths. They should be serialized as the same prime field elements.
#[test]
fn test_poseidon_bn254_x5_fq_hash_bytes_le_smaller_arrays_random() {
    for nr_inputs in 1..12 {
        let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();
        for smaller_arr_len in 1..31 {
            let inputs: Vec<Vec<u8>> = (0..nr_inputs)
                .map(|_| {
                    let rng = rand::thread_rng();
                    rng.sample_iter(rand::distributions::Standard)
                        .take(smaller_arr_len)
                        .collect()
                })
                .collect();
            let inputs: Vec<&[u8]> = inputs.iter().map(|v| &v[..]).collect();
            let hash1 = hasher.hash_bytes_le(inputs.as_slice()).unwrap();

            for greater_arr_len in smaller_arr_len + 1..32 {
                let inputs: Vec<Vec<u8>> = inputs
                    .iter()
                    .map(|input| {
                        [input.to_vec(), vec![0u8; greater_arr_len - smaller_arr_len]].concat()
                    })
                    .collect();
                let inputs: Vec<&[u8]> = inputs.iter().map(|v| &v[..]).collect();
                let hash = hasher.hash_bytes_le(inputs.as_slice()).unwrap();

                assert_eq!(
                    hash, hash1,
                    "inputs: {nr_inputs}, smaller array length: {smaller_arr_len}, greater array length: {greater_arr_len}"
                );
            }
        }
    }
}

/// Check whether `validate_bytes_length` returns an error when an input is a
/// byte slice with greater number of elements than indicated by the modulus.
#[test]
fn test_poseidon_bn254_x5_fq_validate_bytes_length() {
    for i in 1..32 {
        let input = vec![1u8; i];
        let res = validate_bytes_length::<Fr>(&input).unwrap();
        assert_eq!(res, &input);
    }

    for i in 33..64 {
        let input = vec![1u8; i];
        let res = validate_bytes_length::<Fr>(&input);
        assert!(res.is_err());
    }
}

/// Check whether `validate_bytes_length` returns an error when an input is a
/// byte slice with greater number of elements than indicated by the modulus.
/// Randomize the length.
#[test]
fn test_poseidon_bn254_x5_fq_validate_bytes_length_fuzz() {
    let mut rng = rand::thread_rng();

    for _ in 0..100 {
        let len = rng.gen_range(33..524_288_000); // Maximum 500 MB.
        let input = vec![1u8; len];
        let res = validate_bytes_length::<Fr>(&input);

        assert!(res.is_err());
    }
}

/// Checks whether hashes generated by [`PoseidonHasher::hash`],
/// [`PoseidonBytesHasher::hash_bytes_be`] and [`PoseidonBytesHasher::hash_bytes_le`]
/// are the same.
#[test]
fn test_poseidon_bn254_x5_fq_bytes() {
    let mut rng = rand::thread_rng();

    for _ in 0..100 {
        for nr_inputs in 1..12 {
            let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

            // Hash prime field elements.
            let mut inputs = Vec::with_capacity(nr_inputs);
            for _ in 0..nr_inputs {
                inputs.push(Fr::rand(&mut rng));
            }
            let res = hasher.hash(&inputs).unwrap();

            // Hash big-endian bytes. Ensure that the result is the same.
            let inputs_bytes_be: Vec<_> = inputs
                .iter()
                .map(|i| i.into_bigint().to_bytes_be())
                .collect();
            let inputs_bytes_be: Vec<&[u8]> = inputs_bytes_be.iter().map(|v| &v[..]).collect();
            let res_bytes_be = hasher.hash_bytes_be(&inputs_bytes_be).unwrap();
            assert_eq!(res.into_bigint().to_bytes_be(), res_bytes_be);

            // Hash little-endian bytes. Ensure that the result is the same.
            let inputs_bytes_le: Vec<_> = inputs
                .iter()
                .map(|i| i.into_bigint().to_bytes_le())
                .collect();
            let inputs_bytes_le: Vec<&[u8]> = inputs_bytes_le.iter().map(|v| &v[..]).collect();
            let res_bytes_le = hasher.hash_bytes_le(&inputs_bytes_le).unwrap();
            assert_eq!(res.into_bigint().to_bytes_le(), res_bytes_le);
        }
    }
}

macro_rules! test_bytes_to_prime_field_element {
    ($name:ident, $to_bytes_method:ident, $fn:ident) => {
        /// Checks whether `bytes_to_prime_field_element_*` functions:
        ///
        /// * Are converting the valid byte slices appropiately.
        /// * Are throwing an error if the input is greater or equal to the
        ///   modulus.
        #[test]
        fn $name() {
            // Test conversion of random prime field elements from bytes to `F`.
            let mut rng = rand::thread_rng();
            for _ in 0..100 {
                let f = Fr::rand(&mut rng);
                let f = f.into_bigint().$to_bytes_method();
                let res = $fn::<Fr>(&f);
                assert!(res.is_ok());
            }

            let mut lt = Fr::MODULUS;
            lt.sub_with_borrow(&BigInteger256::from(1u64));
            let lt = lt.$to_bytes_method();
            let res = $fn::<Fr>(&lt);

            assert!(res.is_ok());

            let eq = Fr::MODULUS;
            let eq = eq.$to_bytes_method();
            let res = $fn::<Fr>(&eq);

            assert!(res.is_err());

            let mut gt = Fr::MODULUS;
            gt.add_with_carry(&BigInteger256::from(1u64));
            let gt = gt.$to_bytes_method();
            let res = $fn::<Fr>(&gt);

            assert!(res.is_err());
        }
    };
}

test_bytes_to_prime_field_element!(
    test_poseidon_bn254_x5_fq_to_prime_field_element_be,
    to_bytes_be,
    bytes_to_prime_field_element_be
);

test_bytes_to_prime_field_element!(
    test_poseidon_bn254_x5_fq_to_prime_field_element_le,
    to_bytes_le,
    bytes_to_prime_field_element_le
);

macro_rules! test_random_input_same_results {
    ($name:ident, $method:ident) => {
        /// Check whether hashing the same input twice, separately, produces the
        /// same results.
        #[test]
        fn $name() {
            let input = [1u8; 32];

            for nr_inputs in 1..12 {
                let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

                let mut inputs = Vec::with_capacity(nr_inputs);
                for _ in 0..nr_inputs {
                    inputs.push(input.as_slice());
                }

                let hash1 = hasher.$method(inputs.as_slice()).unwrap();
                let hash2 = hasher.$method(inputs.as_slice()).unwrap();

                assert_eq!(hash1, hash2);
            }
        }
    };
}

test_random_input_same_results!(
    test_poseidon_bn254_x5_fq_hash_bytes_be_random_input_same_results,
    hash_bytes_be
);

test_random_input_same_results!(
    test_poseidon_bn254_x5_fq_hash_bytes_le_random_input_same_results,
    hash_bytes_le
);

macro_rules! test_invalid_input_length {
    ($name:ident, $method:ident) => {
        /// Checks whether hashing byte slices with number of elements larger
        /// than indicated by modulus returns an error.
        #[test]
        fn $name() {
            let mut rng = rand::thread_rng();

            for _ in 0..100 {
                let len = rng.gen_range(33..524_288_000); // Maximum 500 MB.
                let input = vec![1u8; len];

                for nr_inputs in 1..12 {
                    let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

                    let mut inputs = Vec::with_capacity(nr_inputs);
                    for _ in 0..nr_inputs {
                        inputs.push(input.as_slice());
                    }

                    let hash = hasher.$method(inputs.as_slice());
                    assert_eq!(
                        hash,
                        Err(PoseidonError::InvalidInputLength {
                            len,
                            modulus_bytes_len: 32,
                        })
                    );
                }
            }
        }
    };
}

test_invalid_input_length!(
    test_poseidon_bn254_x5_fq_hash_bytes_be_invalid_input_length,
    hash_bytes_be
);

test_invalid_input_length!(
    test_poseidon_bn254_x5_fq_hash_bytes_le_invalid_input_length,
    hash_bytes_le
);

macro_rules! test_fuzz_input_gte_field_size {
    ($name:ident, $method:ident, $to_bytes_method:ident) => {
        /// Checks whether hashing a byte slice representing an element larger
        /// than modulus returns an error.
        #[test]
        fn $name() {
            let mut greater_than_field_size = Fr::MODULUS;
            let mut rng = rand::thread_rng();
            let random_number = rng.gen_range(0u64..1_000_000u64);
            greater_than_field_size.add_with_carry(&BigInteger256::from(random_number));
            let greater_than_field_size = greater_than_field_size.$to_bytes_method();

            assert_eq!(greater_than_field_size.len(), 32);

            for nr_inputs in 1..12 {
                let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

                let mut inputs = Vec::with_capacity(nr_inputs);
                for _ in 0..nr_inputs {
                    inputs.push(&greater_than_field_size[..]);
                }

                let hash = hasher.$method(inputs.as_slice());
                assert_eq!(hash, Err(PoseidonError::InputLargerThanModulus));
            }
        }
    };
}

test_fuzz_input_gte_field_size!(
    test_fuzz_poseidon_bn254_fq_hash_bytes_be_input_gt_field_size,
    hash_bytes_be,
    to_bytes_be
);

test_fuzz_input_gte_field_size!(
    test_fuzz_poseidon_bn254_fq_hash_bytes_le_input_gt_field_size,
    hash_bytes_le,
    to_bytes_le
);

macro_rules! test_input_gte_field_size {
    ($name:ident, $method:ident, $greater_than_field_size:expr) => {
        /// Checks whether hashing a byte slice representing an element larger
        /// than modulus returns an error.
        #[test]
        fn $name() {
            for nr_inputs in 1..12 {
                let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

                let mut inputs = Vec::with_capacity(nr_inputs);
                for _ in 0..nr_inputs {
                    inputs.push(&$greater_than_field_size[..]);
                }

                let hash = hasher.$method(inputs.as_slice());
                assert_eq!(hash, Err(PoseidonError::InputLargerThanModulus));
            }
        }
    };
}

test_input_gte_field_size!(
    test_poseidon_bn254_fq_hash_bytes_be_input_gt_field_size_our_check,
    hash_bytes_be,
    [
        216, 137, 85, 159, 239, 194, 107, 138, 254, 68, 21, 16, 165, 41, 64, 148, 208, 198, 201,
        59, 220, 102, 142, 81, 49, 251, 174, 183, 183, 182, 4, 32,
    ]
);

test_input_gte_field_size!(
    test_poseidon_bn254_fq_hash_bytes_le_input_gt_field_size_our_check,
    hash_bytes_le,
    [
        32, 4, 182, 183, 183, 174, 251, 49, 81, 142, 102, 220, 59, 201, 198, 208, 148, 64, 41, 165,
        16, 21, 68, 254, 138, 107, 194, 239, 159, 85, 137, 216,
    ]
);

test_input_gte_field_size!(
    test_poseidon_bn254_fq_hash_bytes_be_input_gt_field_size,
    hash_bytes_be,
    [
        48, 100, 78, 114, 225, 49, 160, 41, 184, 80, 69, 182, 129, 129, 88, 93, 40, 51, 232, 72,
        121, 185, 112, 145, 67, 225, 245, 147, 240, 0, 0, 2
    ]
);

test_input_gte_field_size!(
    test_poseidon_bn254_fq_hash_bytes_le_input_gt_field_size,
    hash_bytes_le,
    [
        2, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129,
        182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48
    ]
);

macro_rules! test_input_eq_field_size {
    ($name:ident, $method:ident, $to_bytes_method:ident) => {
        /// Checks whether hashing a byte slice representing a modulus returns
        /// an error.
        #[test]
        fn $name() {
            let mut hasher = Poseidon::<Fr>::new_circom(1).unwrap();
            let input = Fr::MODULUS.$to_bytes_method();
            let hash = hasher.$method(&[&input]);
            assert_eq!(hash, Err(PoseidonError::InputLargerThanModulus));
        }
    };
}

test_input_eq_field_size!(test_input_eq_field_size_be, hash_bytes_be, to_bytes_be);
test_input_eq_field_size!(test_input_eq_field_size_le, hash_bytes_le, to_bytes_le);

/// Checks that endianness is honored correctly and produces expected hashes.
#[test]
fn test_endianness() {
    let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
    let le_input: &[u8] = &[0, 0, 0, 1];
    let be_input: &[u8] = &[1, 0, 0, 0];

    let hash1 = hasher.hash_bytes_le(&[le_input, le_input]).unwrap();
    let mut hash2 = hasher.hash_bytes_be(&[be_input, be_input]).unwrap();

    assert_ne!(hash1, hash2);

    // Make it little-endian.
    hash2.reverse();

    assert_eq!(hash1, hash2);

    let le_input: &[u8] = &[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ];
    let be_input: &[u8] = &[
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let hash3 = hasher.hash_bytes_le(&[le_input, le_input]).unwrap();
    let mut hash4 = hasher.hash_bytes_be(&[be_input, be_input]).unwrap();

    assert_ne!(hash3, hash4);

    // Make it little-endian.
    hash4.reverse();

    // Compare the latest hashes.
    assert_eq!(hash3, hash4);

    let one = 1u64;
    let le_input = one.to_le_bytes();
    let be_input = one.to_be_bytes();

    let hash5 = hasher.hash_bytes_le(&[&le_input, &le_input]).unwrap();
    let mut hash6 = hasher.hash_bytes_be(&[&be_input, &be_input]).unwrap();

    assert_ne!(hash5, hash6);

    // Make it little-endian,
    hash6.reverse();

    // Compare the latest hashes.
    assert_eq!(hash5, hash6);
}

/// Checks whether providing an empty input results in an error.
#[test]
fn test_empty_input() {
    let empty: &[u8] = &[];
    let non_empty = &[1u8; 32];

    // All inputs empty.
    for nr_inputs in 1..12 {
        let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

        let mut inputs = Vec::with_capacity(nr_inputs);
        for _ in 0..nr_inputs {
            inputs.push(empty);
        }

        let hash = hasher.hash_bytes_be(inputs.as_slice());
        assert_eq!(hash, Err(PoseidonError::EmptyInput));

        let hash = hasher.hash_bytes_le(inputs.as_slice());
        assert_eq!(hash, Err(PoseidonError::EmptyInput));
    }

    // One empty input.
    for nr_inputs in 1..12 {
        let mut hasher = Poseidon::<Fr>::new_circom(nr_inputs).unwrap();

        let mut inputs = Vec::with_capacity(nr_inputs);
        for _ in 0..(nr_inputs - 1) {
            inputs.push(non_empty.as_slice());
        }
        inputs.push(empty);

        let hash = hasher.hash_bytes_be(inputs.as_slice());
        assert_eq!(hash, Err(PoseidonError::EmptyInput));

        let hash = hasher.hash_bytes_le(inputs.as_slice());
        assert_eq!(hash, Err(PoseidonError::EmptyInput));
    }
}

// test cases were created with circomlibjs poseidon([1, ...]) for 1 to 16 inputs
const TEST_CASES: [[u8; 32]; 12] = [
    [
        41, 23, 97, 0, 234, 169, 98, 189, 193, 254, 108, 101, 77, 106, 60, 19, 14, 150, 164, 209,
        22, 139, 51, 132, 139, 137, 125, 197, 2, 130, 1, 51,
    ],
    [
        0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167, 138,
        203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129,
    ],
    [
        2, 192, 6, 110, 16, 167, 42, 189, 43, 51, 195, 178, 20, 203, 62, 129, 188, 177, 182, 227,
        9, 97, 205, 35, 194, 2, 177, 134, 115, 191, 37, 67,
    ],
    [
        8, 44, 156, 55, 10, 13, 36, 244, 65, 111, 188, 65, 74, 55, 104, 31, 120, 68, 45, 39, 216,
        99, 133, 153, 28, 23, 214, 252, 12, 75, 125, 113,
    ],
    [
        16, 56, 150, 5, 174, 104, 141, 79, 20, 219, 133, 49, 34, 196, 125, 102, 168, 3, 199, 43,
        65, 88, 156, 177, 191, 134, 135, 65, 178, 6, 185, 187,
    ],
    [
        42, 115, 246, 121, 50, 140, 62, 171, 114, 74, 163, 229, 189, 191, 80, 179, 144, 53, 215,
        114, 159, 19, 91, 151, 9, 137, 15, 133, 197, 220, 94, 118,
    ],
    [
        34, 118, 49, 10, 167, 243, 52, 58, 40, 66, 20, 19, 157, 157, 169, 89, 190, 42, 49, 178,
        199, 8, 165, 248, 25, 84, 178, 101, 229, 58, 48, 184,
    ],
    [
        23, 126, 20, 83, 196, 70, 225, 176, 125, 43, 66, 51, 66, 81, 71, 9, 92, 79, 202, 187, 35,
        61, 35, 11, 109, 70, 162, 20, 217, 91, 40, 132,
    ],
    [
        14, 143, 238, 47, 228, 157, 163, 15, 222, 235, 72, 196, 46, 187, 68, 204, 110, 231, 5, 95,
        97, 251, 202, 94, 49, 59, 138, 95, 202, 131, 76, 71,
    ],
    [
        46, 196, 198, 94, 99, 120, 171, 140, 115, 48, 133, 79, 74, 112, 119, 193, 255, 146, 96,
        228, 72, 133, 196, 184, 29, 209, 49, 173, 58, 134, 205, 150,
    ],
    [
        0, 113, 61, 65, 236, 166, 53, 241, 23, 212, 236, 188, 235, 95, 58, 102, 220, 65, 66, 235,
        112, 181, 103, 101, 188, 53, 143, 27, 236, 64, 187, 155,
    ],
    [
        20, 57, 11, 224, 186, 239, 36, 155, 212, 124, 101, 221, 172, 101, 194, 229, 46, 133, 19,
        192, 129, 193, 205, 114, 201, 128, 6, 9, 142, 154, 143, 190,
    ],
];

#[test]
fn test_circom_1_to_12_inputs() {
    let mut inputs = Vec::new();
    let value = [vec![0u8; 31], vec![1u8]].concat();
    for i in 1..13 {
        inputs.push(value.as_slice());
        let mut hasher = Poseidon::<Fr>::new_circom(i).unwrap();
        let hash = hasher.hash_bytes_be(&inputs[..]).unwrap();
        assert_eq!(hash, TEST_CASES[i - 1]);
    }
    let mut inputs = Vec::new();
    let value = [vec![0u8; 31], vec![2u8]].concat();
    for i in 1..13 {
        inputs.push(value.as_slice());
        let mut hasher = Poseidon::<Fr>::new_circom(i).unwrap();
        let hash = hasher.hash_bytes_be(&inputs[..]).unwrap();
        assert!(hash != TEST_CASES[i - 1]);
    }
}

/// Checks whether creating a hasher for more than 12 inputs results in an
/// error.
#[test]
fn test_circom_solana_t_gt_12_fails() {
    use light_poseidon::PoseidonError;

    let mut inputs = Vec::new();
    let value = [vec![0u8; 31], vec![1u8]].concat();
    for i in 13..16 {
        inputs.push(value.as_slice());
        let hasher = Poseidon::<Fr>::new_circom(i);
        unsafe {
            assert_eq!(
                hasher.unwrap_err_unchecked(),
                PoseidonError::InvalidWidthCircom {
                    width: i + 1,
                    max_limit: 13
                }
            );
        }
    }
}

/// Checks whether crating a hasher for 0 inputs results in an error.
#[test]
fn test_circom_t_0_fails() {
    use light_poseidon::PoseidonError;
    let hasher = Poseidon::<Fr>::new_circom(0);
    unsafe {
        assert_eq!(
            hasher.unwrap_err_unchecked(),
            PoseidonError::InvalidWidthCircom {
                width: 1,
                max_limit: 13
            }
        );
    }
}
