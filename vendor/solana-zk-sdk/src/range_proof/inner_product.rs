//! The inner-product proof protocol.
//!
//! This module implements the inner-product proof protocol described in Section 3 of the
//! Bulletproofs [`paper`] (also described in [`notes`]). . The protocol is a recursive
//! argument that allows a prover to convince a verifier that the inner product of two
//! secret vectors `a` and `b` is a certain public value `c`.
//!
//! [`paper`]: https://eprint.iacr.org/2017/1066
//! [`notes`]: https://doc-internal.dalek.rs/bulletproofs/notes/inner_product_proof/index.html

use {
    crate::{
        range_proof::{
            errors::{RangeProofGenerationError, RangeProofVerificationError},
            util,
        },
        transcript::TranscriptProtocol,
    },
    core::iter,
    curve25519_dalek::{
        ristretto::{CompressedRistretto, RistrettoPoint},
        scalar::Scalar,
        traits::{MultiscalarMul, VartimeMultiscalarMul},
    },
    merlin::Transcript,
    std::borrow::Borrow,
};

/// An inner-product proof.
///
/// The proof consists of `log(n)` pairs of compressed Ristretto points, and two scalars.
/// This corresponds to the `L_i`, `R_i` values and the final `a`, `b` scalars in Protocol 2
/// of the Bulletproofs paper.
#[allow(non_snake_case)]
#[derive(Clone)]
pub struct InnerProductProof {
    pub(crate) L_vec: Vec<CompressedRistretto>, // 32 * log(bit_length)
    pub(crate) R_vec: Vec<CompressedRistretto>, // 32 * log(bit_length)
    pub(crate) a: Scalar,                       // 32 bytes
    pub(crate) b: Scalar,                       // 32 bytes
}

#[allow(non_snake_case)]
impl InnerProductProof {
    /// Creates an inner-product proof.
    ///
    /// This function implements Protocol 2 from the Bulletproofs paper, a recursive
    /// argument to prove knowledge of two vectors `a` and `b` such that `<a,b> = c`.
    /// The length of the vectors must be a power of two.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        Q: &RistrettoPoint,
        G_factors: &[Scalar],
        H_factors: &[Scalar],
        mut G_vec: Vec<RistrettoPoint>,
        mut H_vec: Vec<RistrettoPoint>,
        mut a_vec: Vec<Scalar>,
        mut b_vec: Vec<Scalar>,
        transcript: &mut Transcript,
    ) -> Result<Self, RangeProofGenerationError> {
        // Create slices G, H, a, b backed by their respective
        // vectors.  This lets us reslice as we compress the lengths
        // of the vectors in the main loop below.
        let mut G = &mut G_vec[..];
        let mut H = &mut H_vec[..];
        let mut a = &mut a_vec[..];
        let mut b = &mut b_vec[..];

        let mut n = G.len();

        // All of the input vectors must have the same length.
        if G.len() != n
            || H.len() != n
            || a.len() != n
            || b.len() != n
            || G_factors.len() != n
            || H_factors.len() != n
        {
            return Err(RangeProofGenerationError::GeneratorLengthMismatch);
        }

        // All of the input vectors must have a length that is a power of two.
        if !n.is_power_of_two() {
            return Err(RangeProofGenerationError::InvalidBitSize);
        }

        transcript.inner_product_proof_domain_separator(n as u64);

        let lg_n = n.next_power_of_two().trailing_zeros() as usize;
        let mut L_vec = Vec::with_capacity(lg_n);
        let mut R_vec = Vec::with_capacity(lg_n);

        // This is an optimization: the first round of the protocol is unrolled from the
        // main loop to handle the `G_factors` and `H_factors` more efficiently using
        // a single multiscalar multiplication. Subsequent rounds use a simplified loop.
        if n != 1 {
            n = n.checked_div(2).unwrap();
            let (a_L, a_R) = a.split_at_mut(n);
            let (b_L, b_R) = b.split_at_mut(n);
            let (G_L, G_R) = G.split_at_mut(n);
            let (H_L, H_R) = H.split_at_mut(n);

            // Compute the cross terms c_L and c_R
            let c_L = util::inner_product(a_L, b_R)
                .ok_or(RangeProofGenerationError::InnerProductLengthMismatch)?;
            let c_R = util::inner_product(a_R, b_L)
                .ok_or(RangeProofGenerationError::InnerProductLengthMismatch)?;

            // Compute L and R points for this round
            // L = <a_L, G_R> + <b_R, H_L> + c_L * Q
            let L = RistrettoPoint::multiscalar_mul(
                a_L.iter()
                    // `n` was previously divided in half and therefore, it cannot overflow.
                    .zip(G_factors[n..n.checked_mul(2).unwrap()].iter())
                    .map(|(a_L_i, g)| a_L_i * g)
                    .chain(
                        b_R.iter()
                            .zip(H_factors[0..n].iter())
                            .map(|(b_R_i, h)| b_R_i * h),
                    )
                    .chain(iter::once(c_L)),
                G_R.iter().chain(H_L.iter()).chain(iter::once(Q)),
            )
            .compress();

            // R = <a_R, G_L> + <b_L, H_R> + c_R * Q
            let R = RistrettoPoint::multiscalar_mul(
                a_R.iter()
                    .zip(G_factors[0..n].iter())
                    .map(|(a_R_i, g)| a_R_i * g)
                    .chain(
                        b_L.iter()
                            .zip(H_factors[n..n.checked_mul(2).unwrap()].iter())
                            .map(|(b_L_i, h)| b_L_i * h),
                    )
                    .chain(iter::once(c_R)),
                G_L.iter().chain(H_R.iter()).chain(iter::once(Q)),
            )
            .compress();

            L_vec.push(L);
            R_vec.push(R);

            transcript.append_point(b"L", &L);
            transcript.append_point(b"R", &R);

            let u = transcript.challenge_scalar(b"u");
            let u_inv = u.invert();

            for i in 0..n {
                a_L[i] = a_L[i] * u + u_inv * a_R[i];
                b_L[i] = b_L[i] * u_inv + u * b_R[i];
                G_L[i] = RistrettoPoint::multiscalar_mul(
                    &[
                        u_inv * G_factors[i],
                        u * G_factors[n.checked_add(i).unwrap()],
                    ],
                    &[G_L[i], G_R[i]],
                );
                H_L[i] = RistrettoPoint::multiscalar_mul(
                    &[
                        u * H_factors[i],
                        u_inv * H_factors[n.checked_add(i).unwrap()],
                    ],
                    &[H_L[i], H_R[i]],
                )
            }

            a = a_L;
            b = b_L;
            G = G_L;
            H = H_L;
        }

        // Main recursive loop
        while n != 1 {
            n = n.checked_div(2).unwrap();
            let (a_L, a_R) = a.split_at_mut(n);
            let (b_L, b_R) = b.split_at_mut(n);
            let (G_L, G_R) = G.split_at_mut(n);
            let (H_L, H_R) = H.split_at_mut(n);

            // Compute the cross terms c_L and c_R
            let c_L = util::inner_product(a_L, b_R)
                .ok_or(RangeProofGenerationError::InnerProductLengthMismatch)?;
            let c_R = util::inner_product(a_R, b_L)
                .ok_or(RangeProofGenerationError::InnerProductLengthMismatch)?;

            // Compute L and R points for this round
            // L = <a_L, G_R> + <b_R, H_L> + c_L * Q
            let L = RistrettoPoint::multiscalar_mul(
                a_L.iter().chain(b_R.iter()).chain(iter::once(&c_L)),
                G_R.iter().chain(H_L.iter()).chain(iter::once(Q)),
            )
            .compress();

            // R = <a_R, G_L> + <b_L, H_R> + c_R * Q
            let R = RistrettoPoint::multiscalar_mul(
                a_R.iter().chain(b_L.iter()).chain(iter::once(&c_R)),
                G_L.iter().chain(H_R.iter()).chain(iter::once(Q)),
            )
            .compress();

            L_vec.push(L);
            R_vec.push(R);

            transcript.append_point(b"L", &L);
            transcript.append_point(b"R", &R);

            let u = transcript.challenge_scalar(b"u");
            let u_inv = u.invert();

            for i in 0..n {
                a_L[i] = a_L[i] * u + u_inv * a_R[i];
                b_L[i] = b_L[i] * u_inv + u * b_R[i];
                G_L[i] = RistrettoPoint::multiscalar_mul(&[u_inv, u], &[G_L[i], G_R[i]]);
                H_L[i] = RistrettoPoint::multiscalar_mul(&[u, u_inv], &[H_L[i], H_R[i]]);
            }

            a = a_L;
            b = b_L;
            G = G_L;
            H = H_L;
        }

        Ok(InnerProductProof {
            L_vec,
            R_vec,
            a: a[0],
            b: b[0],
        })
    }

    /// Computes the verification scalars for a single inner product proof.
    ///
    /// This is a helper function for the verifier, implementing the logic from
    /// Section 3.1 of the paper. It computes the scalars `u_i^2`, `u_i^-2`, and `s_i`
    /// which are needed for the final, single multiscalar multiplication check.
    #[allow(clippy::type_complexity)]
    pub(crate) fn verification_scalars(
        &self,
        n: usize,
        transcript: &mut Transcript,
    ) -> Result<(Vec<Scalar>, Vec<Scalar>, Vec<Scalar>), RangeProofVerificationError> {
        let lg_n = self.L_vec.len();
        if lg_n == 0 || lg_n >= 32 {
            // 4 billion multiplications should be enough for anyone
            // and this check prevents overflow in 1<<lg_n below.
            return Err(RangeProofVerificationError::InvalidBitSize);
        }
        if n != (1_usize.checked_shl(lg_n as u32).unwrap()) {
            return Err(RangeProofVerificationError::InvalidBitSize);
        }

        transcript.inner_product_proof_domain_separator(n as u64);

        // 1. Recompute challenges `u_i` from the proof transcript (`x_i` in the paper).
        let mut challenges = Vec::with_capacity(lg_n);
        for (L, R) in self.L_vec.iter().zip(self.R_vec.iter()) {
            transcript.validate_and_append_point(b"L", L)?;
            transcript.validate_and_append_point(b"R", R)?;
            challenges.push(transcript.challenge_scalar(b"u"));
        }

        // 2. Compute `u_i^-1` for all `i`.
        let mut challenges_inv = challenges.clone();
        // This computes `(u_k * ... * u_1)^-1` and stores `u_i^-1` in `challenges_inv`.
        let allinv = Scalar::batch_invert(&mut challenges_inv);

        // 3. Compute `u_i^2` and `u_i^-2` for all `i`.
        for i in 0..lg_n {
            challenges[i] = challenges[i] * challenges[i];
            challenges_inv[i] = challenges_inv[i] * challenges_inv[i];
        }
        let challenges_sq = challenges;
        let challenges_inv_sq = challenges_inv;

        // 4. Compute `s_i` values inductively, as described in Section 6.2 of the paper.
        let mut s = Vec::with_capacity(n);
        s.push(allinv);
        for i in 1..n {
            let lg_i = 31_u32.checked_sub((i as u32).leading_zeros()).unwrap() as usize;
            let k = 1_usize.checked_shl(lg_i as u32).unwrap();
            // The challenges are stored in "creation order" as [u_lg_n, ..., u_1],
            // so u_{lg_i+1} is indexed by (lg_n - 1) - lg_i
            let u_lg_i_sq = challenges_sq[lg_n
                .checked_sub(1)
                .and_then(|x| x.checked_sub(lg_i))
                .unwrap()];
            s.push(s[i - k] * u_lg_i_sq);
        }

        Ok((challenges_sq, challenges_inv_sq, s))
    }

    /// Verifies an inner product proof.
    ///
    /// This is a standalone verification function for testing. In a real protocol,
    /// the `verification_scalars` method would be used to integrate the check into
    /// a larger, single multiscalar multiplication.
    #[allow(clippy::too_many_arguments)]
    pub fn verify<IG, IH>(
        &self,
        n: usize,
        G_factors: IG,
        H_factors: IH,
        P: &RistrettoPoint,
        Q: &RistrettoPoint,
        G: &[RistrettoPoint],
        H: &[RistrettoPoint],
        transcript: &mut Transcript,
    ) -> Result<(), RangeProofVerificationError>
    where
        IG: IntoIterator,
        IG::Item: Borrow<Scalar>,
        IH: IntoIterator,
        IH::Item: Borrow<Scalar>,
    {
        let (u_sq, u_inv_sq, s) = self.verification_scalars(n, transcript)?;

        let g_times_a_times_s = G_factors
            .into_iter()
            .zip(s.iter())
            .map(|(g_i, s_i)| (self.a * s_i) * g_i.borrow())
            .take(G.len());

        // 1/s[i] is s[!i], and !i runs from n-1 to 0 as i runs from 0 to n-1
        let inv_s = s.iter().rev();

        let h_times_b_div_s = H_factors
            .into_iter()
            .zip(inv_s)
            .map(|(h_i, s_i_inv)| (self.b * s_i_inv) * h_i.borrow());

        let neg_u_sq = u_sq.iter().map(|ui| -ui);
        let neg_u_inv_sq = u_inv_sq.iter().map(|ui| -ui);

        let Ls = self
            .L_vec
            .iter()
            .map(|p| {
                p.decompress()
                    .ok_or(RangeProofVerificationError::Deserialization)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let Rs = self
            .R_vec
            .iter()
            .map(|p| {
                p.decompress()
                    .ok_or(RangeProofVerificationError::Deserialization)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // This check implements the verification equation from Section 3.1 of the paper.
        // P' = P + <L, u^2> + <R, u^-2>
        // We check that P' = g^(a*s) * h^(b*s^-1) * Q^(a*b)
        let expect_P = RistrettoPoint::vartime_multiscalar_mul(
            iter::once(self.a * self.b)
                .chain(g_times_a_times_s)
                .chain(h_times_b_div_s)
                .chain(neg_u_sq)
                .chain(neg_u_inv_sq),
            iter::once(Q)
                .chain(G.iter())
                .chain(H.iter())
                .chain(Ls.iter())
                .chain(Rs.iter()),
        );

        if expect_P == *P {
            Ok(())
        } else {
            Err(RangeProofVerificationError::AlgebraicRelation)
        }
    }

    /// Returns the size in bytes required to serialize the inner product proof.
    ///
    /// For vectors of length `n`, the proof size is `(2*log2(n) + 2) * 32` bytes.
    pub fn serialized_size(&self) -> usize {
        (self.L_vec.len() * 2 + 2) * 32
    }

    /// Serializes the proof into a byte array.
    /// The layout of the inner product proof is:
    /// - `log(n)` compressed Ristretto points for L_vec
    /// - `log(n)` compressed Ristretto points for R_vec
    /// - a scalar `a`
    /// - a scalar `b`
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.serialized_size());
        for (l, r) in self.L_vec.iter().zip(self.R_vec.iter()) {
            buf.extend_from_slice(l.as_bytes());
            buf.extend_from_slice(r.as_bytes());
        }
        buf.extend_from_slice(self.a.as_bytes());
        buf.extend_from_slice(self.b.as_bytes());
        buf
    }

    /// Deserializes the proof from a byte slice.
    ///
    /// Returns an error if the slice is malformed or does not represent a
    /// canonical inner product proof. The function checks for the following
    /// failure conditions:
    ///
    /// * The slice's length is not a multiple of 32.
    /// * The proof contains an invalid number of elements (e.g., no `a` and `b`
    ///   scalars, or an odd number of point commitments).
    /// * The number of L/R pairs (`log(n)`) is 32 or greater, which is
    ///   considered an invalid proof size that could cause overflows.
    /// * The bytes for the final `a` or `b` scalars are not canonical representations
    ///   modulo the Ristretto group order.
    ///
    /// Note: This function does not check if the `L` and `R` points are valid
    /// Ristretto points on the curve. The verifier should handle this by attempting
    /// to decompress them, which will fail for invalid points.
    pub fn from_bytes(slice: &[u8]) -> Result<InnerProductProof, RangeProofVerificationError> {
        let b = slice.len();
        if b % 32 != 0 {
            return Err(RangeProofVerificationError::Deserialization);
        }
        let num_elements = b / 32;
        if num_elements < 2 {
            return Err(RangeProofVerificationError::Deserialization);
        }
        if (num_elements - 2) % 2 != 0 {
            return Err(RangeProofVerificationError::Deserialization);
        }
        let lg_n = (num_elements - 2) / 2;
        if lg_n >= 32 {
            return Err(RangeProofVerificationError::Deserialization);
        }

        let mut L_vec: Vec<CompressedRistretto> = Vec::with_capacity(lg_n);
        let mut R_vec: Vec<CompressedRistretto> = Vec::with_capacity(lg_n);
        for i in 0..lg_n {
            let pos = 2 * i * 32;
            L_vec.push(CompressedRistretto(util::read32(&slice[pos..])));
            R_vec.push(CompressedRistretto(util::read32(&slice[pos + 32..])));
        }

        let pos = 2 * lg_n * 32;
        let a = Scalar::from_canonical_bytes(util::read32(&slice[pos..]))
            .into_option()
            .ok_or(RangeProofVerificationError::Deserialization)?;
        let b = Scalar::from_canonical_bytes(util::read32(&slice[pos + 32..]))
            .into_option()
            .ok_or(RangeProofVerificationError::Deserialization)?;

        Ok(InnerProductProof { L_vec, R_vec, a, b })
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*, crate::range_proof::generators::RangeProofGens, rand::rngs::OsRng, sha3::Sha3_512,
    };

    #[test]
    #[allow(non_snake_case)]
    fn test_basic_correctness() {
        let n = 32;

        let bp_gens = RangeProofGens::new(n).unwrap();
        let G: Vec<RistrettoPoint> = bp_gens.G(n).cloned().collect();
        let H: Vec<RistrettoPoint> = bp_gens.H(n).cloned().collect();

        let Q = RistrettoPoint::hash_from_bytes::<Sha3_512>(b"test point");

        let a: Vec<_> = (0..n).map(|_| Scalar::random(&mut OsRng)).collect();
        let b: Vec<_> = (0..n).map(|_| Scalar::random(&mut OsRng)).collect();
        let c = util::inner_product(&a, &b).unwrap();

        let G_factors: Vec<Scalar> = iter::repeat_n(Scalar::ONE, n).collect();

        let y_inv = Scalar::random(&mut OsRng);
        let H_factors: Vec<Scalar> = util::exp_iter(y_inv).take(n).collect();

        // P would be determined upstream, but we need a correct P to check the proof.
        //
        // To generate P = <a,G> + <b,H'> + <a,b> Q, compute
        //             P = <a,G> + <b',H> + <a,b> Q,
        // where b' = b \circ y^(-n)
        let b_prime = b.iter().zip(util::exp_iter(y_inv)).map(|(bi, yi)| bi * yi);
        // a.iter() has Item=&Scalar, need Item=Scalar to chain with b_prime
        let a_prime = a.iter().cloned();

        let P = RistrettoPoint::vartime_multiscalar_mul(
            a_prime.chain(b_prime).chain(iter::once(c)),
            G.iter().chain(H.iter()).chain(iter::once(&Q)),
        );

        let mut prover_transcript = Transcript::new(b"innerproducttest");
        let mut verifier_transcript = Transcript::new(b"innerproducttest");

        let proof = InnerProductProof::new(
            &Q,
            &G_factors,
            &H_factors,
            G.clone(),
            H.clone(),
            a.clone(),
            b.clone(),
            &mut prover_transcript,
        )
        .unwrap();

        assert!(proof
            .verify(
                n,
                iter::repeat_n(Scalar::ONE, n),
                util::exp_iter(y_inv).take(n),
                &P,
                &Q,
                &G,
                &H,
                &mut verifier_transcript,
            )
            .is_ok());

        let proof = InnerProductProof::from_bytes(proof.to_bytes().as_slice()).unwrap();
        let mut verifier_transcript = Transcript::new(b"innerproducttest");
        assert!(proof
            .verify(
                n,
                iter::repeat_n(Scalar::ONE, n),
                util::exp_iter(y_inv).take(n),
                &P,
                &Q,
                &G,
                &H,
                &mut verifier_transcript,
            )
            .is_ok());
    }
}
