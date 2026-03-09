//! **light-poseidon** is a [Poseidon](https://eprint.iacr.org/2019/458) hash
//! implementation in Rust created for [Light Protocol](https://www.lightprotocol.com/).
//!
//! # Parameters
//!
//! The library provides pre-generated parameters over the BN254 curve, however
//! it can work with any parameters provided as long as developers take care
//! of generating the round constants.
//!
//! Parameters provided by the library are:
//!
//! * *x^5* S-boxes
//! * width - *2 ≤ t ≤ 13*
//! * inputs - *1 ≤ n ≤ 12*
//! * 8 full rounds and partial rounds depending on *t*: *[56, 57, 56, 60, 60, 63, 64, 63, 60, 66, 60, 65]*
//!
//! The parameters can be generated with:
//!
//! ```bash
//! cargo xtask generate-poseidon-parameters
//! ````
//!
//! # Output type
//!
//! [`Poseidon`](crate::Poseidon) type implements two traits which serve the purpose
//! of returning the calculated hash in different representations:
//!
//! * [`PoseidonBytesHasher`](crate::PoseidonBytesHasher) with the
//!   `hash_bytes_be` and `hash_bytes_le` methods which returns a byte array.
//! * [`PoseidonHasher`](crate::PoseidonHasher) with the `hash` method which returns
//!   [`ark_ff::PrimeField`](ark_ff::PrimeField). Might be useful if you want
//!   to immediately process the result with an another library which works with
//!   [`ark_ff::PrimeField`](ark_ff::PrimeField) types.
//!
//! # Examples
//!
//! Example with two simple big-endian byte inputs (converted to field elements)
//! and BN254-based parameters provided by the library, with
//! [`PoseidonBytesHasher`](crate::PoseidonHasher) trait and a byte array
//! result:
//!
//! ```rust
//! use light_poseidon::{Poseidon, PoseidonBytesHasher, parameters::bn254_x5};
//! use ark_bn254::Fr;
//! use ark_ff::{BigInteger, PrimeField};
//!
//! let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
//!
//! let hash = poseidon.hash_bytes_be(&[&[1u8; 32], &[2u8; 32]]).unwrap();
//!
//! println!("{:?}", hash);
//! // Should print:
//! // [
//! //     13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123, 132,
//! //     254, 156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144
//! // ]
//! ```
//!
//! With [`PoseidonHasher`](crate::PoseidonHasher) trait and
//! [`ark_ff::PrimeField`](ark_ff::PrimeField) result:
//!
//! ```rust
//! use light_poseidon::{Poseidon, PoseidonHasher, parameters::bn254_x5};
//! use ark_bn254::Fr;
//! use ark_ff::{BigInteger, PrimeField};
//!
//! let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
//!
//! let input1 = Fr::from_be_bytes_mod_order(&[1u8; 32]);
//! let input2 = Fr::from_be_bytes_mod_order(&[2u8; 32]);
//!
//! let hash = poseidon.hash(&[input1, input2]).unwrap();
//!
//! // Do something with `hash`.
//! ```
//!
//! # Implementation
//!
//! The implementation is compatible with the
//! [original SageMath implementation](https://extgit.iaik.tugraz.at/krypto/hadeshash/-/tree/master/),
//! but it was also inspired by the following ones:
//!
//! * [circomlibjs](https://github.com/iden3/circomlibjs)
//! * [zero-knowledge-gadgets](https://github.com/webb-tools/zero-knowledge-gadgets)
//!
//! # Performance
//!
//! This repository contains a benchmark measuring the performance of this
//! Poseidon implementation for given 1 - 12 random 32 bytes inputs.
//!
//! To run them, simply use:
//!
//! ```bash
//! cargo bench
//! ```
//!
//! This is the result from a host with the following hardware:
//!
//! * 12th Gen Intel® Core™ i7-1260P × 16
//!
//! ```norust
//! poseidon_bn254_x5_1     time:   [17.543 µs 18.303 µs 19.133 µs]
//! Found 9 outliers among 100 measurements (9.00%)
//!   9 (9.00%) high mild
//!
//! poseidon_bn254_x5_2     time:   [25.020 µs 25.866 µs 26.830 µs]
//!
//! poseidon_bn254_x5_3     time:   [36.076 µs 37.549 µs 38.894 µs]
//!
//! poseidon_bn254_x5_4     time:   [50.333 µs 52.598 µs 54.806 µs]
//!
//! poseidon_bn254_x5_5     time:   [64.184 µs 66.324 µs 68.706 µs]
//!
//! poseidon_bn254_x5_6     time:   [87.356 µs 90.259 µs 93.437 µs]
//!
//! poseidon_bn254_x5_7     time:   [120.08 µs 125.26 µs 130.23 µs]
//!
//! poseidon_bn254_x5_8     time:   [134.28 µs 139.65 µs 145.71 µs]
//!
//! poseidon_bn254_x5_9     time:   [161.99 µs 168.93 µs 175.94 µs]
//!
//! poseidon_bn254_x5_10    time:   [208.11 µs 215.27 µs 222.99 µs]
//! Found 1 outliers among 100 measurements (1.00%)
//!   1 (1.00%) high mild
//!
//! poseidon_bn254_x5_11    time:   [239.47 µs 249.05 µs 258.35 µs]
//!
//! poseidon_bn254_x5_12    time:   [295.47 µs 305.80 µs 316.41 µs]
//! ```
//!
//! # Security
//!
//! This library has been audited by [Veridise](https://veridise.com/). You can
//! read the audit report [here](https://github.com/Lightprotocol/light-poseidon/blob/main/assets/audit.pdf).
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField, Zero};
use thiserror::Error;

pub mod parameters;

pub const HASH_LEN: usize = 32;
pub const MAX_X5_LEN: usize = 13;

#[derive(Error, Debug, PartialEq)]
pub enum PoseidonError {
    #[error("Invalid number of inputs: {inputs}. Maximum allowed is {max_limit} ({width} - 1).")]
    InvalidNumberOfInputs {
        inputs: usize,
        max_limit: usize,
        width: usize,
    },
    #[error("Input is an empty slice.")]
    EmptyInput,
    #[error("Invalid length of the input: {len}. The length matching the modulus of the prime field is: {modulus_bytes_len}.")]
    InvalidInputLength {
        len: usize,
        modulus_bytes_len: usize,
    },
    #[error("Failed to convert bytes {bytes:?} into a prime field element")]
    BytesToPrimeFieldElement { bytes: Vec<u8> },
    #[error("Input is larger than the modulus of the prime field.")]
    InputLargerThanModulus,
    #[error("Failed to convert a vector of bytes into an array.")]
    VecToArray,
    #[error("Failed to convert the number of inputs from u64 to u8.")]
    U64Tou8,
    #[error("Failed to convert bytes to BigInt")]
    BytesToBigInt,
    #[error("Invalid width: {width}. Choose a width between 2 and 16 for 1 to 15 inputs.")]
    InvalidWidthCircom { width: usize, max_limit: usize },
}

/// Parameters for the Poseidon hash algorithm.
pub struct PoseidonParameters<F: PrimeField> {
    /// Round constants.
    pub ark: Vec<F>,
    /// MDS matrix.
    pub mds: Vec<Vec<F>>,
    /// Number of full rounds (where S-box is applied to all elements of the
    /// state).
    pub full_rounds: usize,
    /// Number of partial rounds (where S-box is applied only to the first
    /// element of the state).
    pub partial_rounds: usize,
    /// Number of prime fields in the state.
    pub width: usize,
    /// Exponential used in S-box to power elements of the state.
    pub alpha: u64,
}

impl<F: PrimeField> PoseidonParameters<F> {
    pub fn new(
        ark: Vec<F>,
        mds: Vec<Vec<F>>,
        full_rounds: usize,
        partial_rounds: usize,
        width: usize,
        alpha: u64,
    ) -> Self {
        Self {
            ark,
            mds,
            full_rounds,
            partial_rounds,
            width,
            alpha,
        }
    }
}

pub trait PoseidonHasher<F: PrimeField> {
    /// Calculates a Poseidon hash for the given input of prime fields and
    /// returns the result as a prime field.
    ///
    /// # Examples
    ///
    /// Example with two simple big-endian byte inputs (converted to prime
    /// fields) and BN254-based parameters provided by the library.
    ///
    /// ```rust
    /// use light_poseidon::{Poseidon, PoseidonHasher, parameters::bn254_x5};
    /// use ark_bn254::Fr;
    /// use ark_ff::{BigInteger, PrimeField};
    ///
    /// let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
    ///
    /// let input1 = Fr::from_be_bytes_mod_order(&[1u8; 32]);
    /// let input2 = Fr::from_be_bytes_mod_order(&[2u8; 32]);
    ///
    /// let hash = poseidon.hash(&[input1, input2]).unwrap();
    ///
    /// // Do something with `hash`.
    fn hash(&mut self, inputs: &[F]) -> Result<F, PoseidonError>;
}

pub trait PoseidonBytesHasher {
    /// Calculates a Poseidon hash for the given input of big-endian byte slices
    /// and returns the result as a byte array.
    ///
    /// # Examples
    ///
    /// Example with two simple big-endian byte inputs and BN254-based
    /// parameters provided by the library.
    ///
    /// ```rust
    /// use light_poseidon::{Poseidon, PoseidonBytesHasher, parameters::bn254_x5};
    /// use ark_bn254::Fr;
    /// use ark_ff::{BigInteger, PrimeField};
    ///
    /// let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
    ///
    /// let hash = poseidon.hash_bytes_be(&[&[1u8; 32], &[2u8; 32]]).unwrap();
    ///
    /// println!("{:?}", hash);
    /// // Should print:
    /// // [
    /// //     13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123, 132,
    /// //     254, 156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144
    /// // ]
    /// ```
    ///
    /// # Safety
    ///   
    /// Unlike the
    /// [`PrimeField::from_be_bytes_mod_order`](ark_ff::PrimeField::from_be_bytes_mod_order)
    /// and [`Field::from_random_bytes`](ark_ff::Field::from_random_bytes)
    /// methods, this function ensures that the input byte slice's length exactly matches
    /// the modulus size of the prime field. If the size doesn't match, an error is returned.
    ///
    /// This strict check is designed to prevent unexpected behaviors and collisions
    /// that might occur when using `from_be_bytes_mod_order` or `from_random_bytes`,
    /// which simply take a subslice of the input if it's too large, potentially
    /// leading to collisions.
    fn hash_bytes_be(&mut self, inputs: &[&[u8]]) -> Result<[u8; HASH_LEN], PoseidonError>;
    /// Calculates a Poseidon hash for the given input of little-endian byte
    /// slices and returns the result as a byte array.
    ///
    /// # Examples
    ///
    /// Example with two simple little-endian byte inputs and BN254-based
    /// parameters provided by the library.
    ///
    /// ```rust
    /// use light_poseidon::{Poseidon, PoseidonBytesHasher, parameters::bn254_x5};
    /// use ark_bn254::Fr;
    /// use ark_ff::{BigInteger, PrimeField};
    ///
    /// let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
    ///
    /// let hash = poseidon.hash_bytes_le(&[&[1u8; 32], &[2u8; 32]]).unwrap();
    ///
    /// println!("{:?}", hash);
    /// // Should print:
    /// // [
    /// //     144, 25, 130, 41, 200, 53, 231, 38, 27, 206, 162, 156, 254, 132, 123, 32, 25, 99, 242,
    /// //     85, 3, 94, 235, 125, 28, 140, 138, 143, 147, 225, 84, 13
    /// // ]
    /// ```
    ///
    /// # Safety
    ///
    /// Unlike the
    /// [`PrimeField::from_le_bytes_mod_order`](ark_ff::PrimeField::from_le_bytes_mod_order)
    /// and [`Field::from_random_bytes`](ark_ff::Field::from_random_bytes)
    /// methods, this function ensures that the input byte slice's length exactly matches
    /// the modulus size of the prime field. If the size doesn't match, an error is returned.
    ///
    /// This strict check is designed to prevent unexpected behaviors and collisions
    /// that might occur when using `from_be_bytes_mod_order` or `from_random_bytes`,
    /// which simply take a subslice of the input if it's too large, potentially
    /// leading to collisions.
    fn hash_bytes_le(&mut self, inputs: &[&[u8]]) -> Result<[u8; HASH_LEN], PoseidonError>;
}

/// A stateful sponge performing Poseidon hash computation.
pub struct Poseidon<F: PrimeField> {
    params: PoseidonParameters<F>,
    domain_tag: F,
    state: Vec<F>,
}

impl<F: PrimeField> Poseidon<F> {
    /// Returns a new Poseidon hasher based on the given parameters.
    ///
    /// Optionally, a domain tag can be provided. If it is not provided, it
    /// will be set to zero.
    pub fn new(params: PoseidonParameters<F>) -> Self {
        Self::with_domain_tag(params, F::zero())
    }

    fn with_domain_tag(params: PoseidonParameters<F>, domain_tag: F) -> Self {
        let width = params.width;
        Self {
            domain_tag,
            params,
            state: Vec::with_capacity(width),
        }
    }

    #[inline(always)]
    fn apply_ark(&mut self, round: usize) {
        self.state.iter_mut().enumerate().for_each(|(i, a)| {
            let c = self.params.ark[round * self.params.width + i];
            *a += c;
        });
    }

    #[inline(always)]
    fn apply_sbox_full(&mut self) {
        self.state.iter_mut().for_each(|a| {
            *a = a.pow([self.params.alpha]);
        });
    }

    #[inline(always)]
    fn apply_sbox_partial(&mut self) {
        self.state[0] = self.state[0].pow([self.params.alpha]);
    }

    #[inline(always)]
    fn apply_mds(&mut self) {
        self.state = self
            .state
            .iter()
            .enumerate()
            .map(|(i, _)| {
                self.state
                    .iter()
                    .enumerate()
                    .fold(F::zero(), |acc, (j, a)| acc + *a * self.params.mds[i][j])
            })
            .collect();
    }
}

impl<F: PrimeField> PoseidonHasher<F> for Poseidon<F> {
    fn hash(&mut self, inputs: &[F]) -> Result<F, PoseidonError> {
        if inputs.len() != self.params.width - 1 {
            return Err(PoseidonError::InvalidNumberOfInputs {
                inputs: inputs.len(),
                max_limit: self.params.width - 1,
                width: self.params.width,
            });
        }

        self.state.push(self.domain_tag);

        for input in inputs {
            self.state.push(*input);
        }

        let all_rounds = self.params.full_rounds + self.params.partial_rounds;
        let half_rounds = self.params.full_rounds / 2;

        // full rounds + partial rounds
        for round in 0..half_rounds {
            self.apply_ark(round);
            self.apply_sbox_full();
            self.apply_mds();
        }

        for round in half_rounds..half_rounds + self.params.partial_rounds {
            self.apply_ark(round);
            self.apply_sbox_partial();
            self.apply_mds();
        }

        for round in half_rounds + self.params.partial_rounds..all_rounds {
            self.apply_ark(round);
            self.apply_sbox_full();
            self.apply_mds();
        }

        let result = self.state[0];
        self.state.clear();
        Ok(result)
    }
}

macro_rules! impl_hash_bytes {
    ($fn_name:ident, $bytes_to_prime_field_element_fn:ident, $to_bytes_fn:ident) => {
        fn $fn_name(&mut self, inputs: &[&[u8]]) -> Result<[u8; HASH_LEN], PoseidonError> {
            let inputs: Result<Vec<_>, _> = inputs
                .iter()
                .map(|input| validate_bytes_length::<F>(input))
                .collect();
            let inputs = inputs?;
            let inputs: Result<Vec<_>, _> = inputs
                .iter()
                .map(|input| $bytes_to_prime_field_element_fn(input))
                .collect();
            let inputs = inputs?;
            let hash = self.hash(&inputs)?;

            hash.into_bigint()
                .$to_bytes_fn()
                .try_into()
                .map_err(|_| PoseidonError::VecToArray)
        }
    };
}

impl<F: PrimeField> PoseidonBytesHasher for Poseidon<F> {
    impl_hash_bytes!(hash_bytes_le, bytes_to_prime_field_element_le, to_bytes_le);
    impl_hash_bytes!(hash_bytes_be, bytes_to_prime_field_element_be, to_bytes_be);
}

/// Checks whether a slice of bytes is not empty or its length does not exceed
/// the modulus size od the prime field. If it does, an error is returned.
///
/// # Safety
///
/// [`PrimeField::from_be_bytes_mod_order`](ark_ff::PrimeField::from_be_bytes_mod_order)
/// just takes a subslice of the input if it's too large, potentially leading
/// to collisions. The purpose of this function is to prevent them by returning
/// and error. It should be always used before converting byte slices to
/// prime field elements.
pub fn validate_bytes_length<F>(input: &[u8]) -> Result<&[u8], PoseidonError>
where
    F: PrimeField,
{
    let modulus_bytes_len = ((F::MODULUS_BIT_SIZE + 7) / 8) as usize;
    if input.is_empty() {
        return Err(PoseidonError::EmptyInput);
    }
    if input.len() > modulus_bytes_len {
        return Err(PoseidonError::InvalidInputLength {
            len: input.len(),
            modulus_bytes_len,
        });
    }
    Ok(input)
}

macro_rules! impl_bytes_to_prime_field_element {
    ($name:ident, $from_bytes_method:ident, $endianess:expr) => {
        #[doc = "Converts a slice of "]
        #[doc = $endianess]
        #[doc = "-endian bytes into a prime field element, \
                 represented by the [`ark_ff::PrimeField`](ark_ff::PrimeField) trait."]
        pub fn $name<F>(input: &[u8]) -> Result<F, PoseidonError>
        where
            F: PrimeField,
        {
            let element = num_bigint::BigUint::$from_bytes_method(input);
            let element = F::BigInt::try_from(element).map_err(|_| PoseidonError::BytesToBigInt)?;

            // In theory, `F::from_bigint` should also perform a check whether input is
            // larger than modulus (and return `None` if it is), but it's not reliable...
            // To be sure, we check it ourselves.
            if element >= F::MODULUS {
                return Err(PoseidonError::InputLargerThanModulus);
            }
            let element = F::from_bigint(element).ok_or(PoseidonError::InputLargerThanModulus)?;

            Ok(element)
        }
    };
}

impl_bytes_to_prime_field_element!(bytes_to_prime_field_element_le, from_bytes_le, "little");
impl_bytes_to_prime_field_element!(bytes_to_prime_field_element_be, from_bytes_be, "big");

impl<F: PrimeField> Poseidon<F> {
    pub fn new_circom(nr_inputs: usize) -> Result<Poseidon<Fr>, PoseidonError> {
        Self::with_domain_tag_circom(nr_inputs, Fr::zero())
    }

    pub fn with_domain_tag_circom(
        nr_inputs: usize,
        domain_tag: Fr,
    ) -> Result<Poseidon<Fr>, PoseidonError> {
        let width = nr_inputs + 1;
        if width > MAX_X5_LEN {
            return Err(PoseidonError::InvalidWidthCircom {
                width,
                max_limit: MAX_X5_LEN,
            });
        }

        let params = crate::parameters::bn254_x5::get_poseidon_parameters::<Fr>(
            (width).try_into().map_err(|_| PoseidonError::U64Tou8)?,
        )?;
        Ok(Poseidon::<Fr>::with_domain_tag(params, domain_tag))
    }
}
