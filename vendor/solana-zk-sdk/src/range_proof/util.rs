//! Utility functions and structures for the Bulletproofs protocol.
//! This module provides common mathematical operations over scalars and vectors needed to
//! construct and verify Bulletproofs.

use curve25519_dalek::scalar::Scalar;

/// Represents a degree-1 vector polynomial, such as `a + b*x`.
///
/// A vector polynomial is a vector of polynomials. For example:
/// `[a_1 + b_1*x, a_2 + b_2*x, ..., a_n + b_n*x]`
///
/// This struct stores the constant terms (the `a_i`'s) and the linear terms (the `b_i`'s) as two
/// separate vectors of scalars. In the Bulletproofs protocol, this is used to represent the `l(x)`
/// and `r(x)` vector polynomials.
pub struct VecPoly1(pub Vec<Scalar>, pub Vec<Scalar>);

impl VecPoly1 {
    /// Creates a new zero-filled `VecPoly1` of the given size.
    pub fn zero(n: usize) -> Self {
        VecPoly1(vec![Scalar::ZERO; n], vec![Scalar::ZERO; n])
    }

    /// Computes the inner product of two vector polynomials.
    ///
    /// The result is a scalar polynomial of degree 2, `t(x) = t_0 + t_1*x + t_2*x^2`,
    /// which is the inner product `t(x) = <l(x), r(x)>` in the Bulletproofs protocol.
    ///
    /// This function uses Karatsuba's method for efficient polynomial multiplication.
    pub fn inner_product(&self, rhs: &VecPoly1) -> Option<Poly2> {
        // Uses Karatsuba's method
        let l = self;
        let r = rhs;

        let t0 = inner_product(&l.0, &r.0)?;
        let t2 = inner_product(&l.1, &r.1)?;

        let l0_plus_l1 = add_vec(&l.0, &l.1);
        let r0_plus_r1 = add_vec(&r.0, &r.1);

        let t1 = inner_product(&l0_plus_l1, &r0_plus_r1)? - t0 - t2;

        Some(Poly2(t0, t1, t2))
    }

    /// Evaluates the vector polynomial at a given scalar point `x`.
    ///
    /// The result is a vector of scalars, where each element is the evaluation
    /// of the corresponding polynomial at `x`.
    pub fn eval(&self, x: Scalar) -> Vec<Scalar> {
        let n = self.0.len();
        let mut out = vec![Scalar::ZERO; n];
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            out[i] = self.0[i] + self.1[i] * x;
        }
        out
    }
}

/// Represents a degree-2 scalar polynomial `a + b*x + c*x^2`.
///
/// In the Bulletproofs protocol, this represents the `t(x)` polynomial
/// that results from the inner product of `l(x)` and `r(x)`.
pub struct Poly2(pub Scalar, pub Scalar, pub Scalar);

impl Poly2 {
    /// Evaluates the polynomial at a scalar point `x` using Horner's method.
    pub fn eval(&self, x: Scalar) -> Scalar {
        self.0 + x * (self.1 + x * self.2)
    }
}

/// Provides an iterator over the powers of a `Scalar` (e.g., `1, x, x^2, x^3, ...`).
///
/// This struct is created by the `exp_iter` function.
pub struct ScalarExp {
    x: Scalar,
    next_exp_x: Scalar,
}

impl Iterator for ScalarExp {
    type Item = Scalar;

    fn next(&mut self) -> Option<Scalar> {
        let exp_x = self.next_exp_x;
        self.next_exp_x *= self.x;
        Some(exp_x)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

/// Returns an iterator of the powers of `x`, starting with `x^0 = 1`.
pub fn exp_iter(x: Scalar) -> ScalarExp {
    let next_exp_x = Scalar::ONE;
    ScalarExp { x, next_exp_x }
}

/// Computes the component-wise sum of two vectors of scalars.
/// Panics if the lengths of the vectors two do not match.
pub fn add_vec(a: &[Scalar], b: &[Scalar]) -> Vec<Scalar> {
    if a.len() != b.len() {
        panic!("lengths of vectors don't match for vector addition");
    }
    let mut out = vec![Scalar::ZERO; b.len()];
    for i in 0..a.len() {
        out[i] = a[i] + b[i];
    }
    out
}

/// Returns the first 32 bytes of a slice as a fixed-size array.
/// Panics if the slice is less than 32 bytes long.
pub fn read32(data: &[u8]) -> [u8; 32] {
    let mut buf32 = [0u8; 32];
    buf32[..].copy_from_slice(&data[..32]);
    buf32
}

/// Computes the inner product of two vectors of scalars.
///
/// The inner product is defined as:
/// `<a, b> = a_1*b_1 + a_2*b_2 + ... + a_n*b_n`
///
/// Returns `None` if the vectors have different lengths.
pub fn inner_product(a: &[Scalar], b: &[Scalar]) -> Option<Scalar> {
    let mut out = Scalar::ZERO;
    if a.len() != b.len() {
        return None;
    }
    for i in 0..a.len() {
        out += a[i] * b[i];
    }
    Some(out)
}

/// Computes the sum of powers of a scalar `x`.
///
/// Calculates `1 + x + x^2 + ... + x^(n-1)`.
///
/// In the Bulletproofs protocol, vector spaces are padded to a power of two `n`,
/// so this function uses an efficient algorithm for that case. If `n` is not a
/// power of two, it falls back to a slower method.
pub fn sum_of_powers(x: &Scalar, n: usize) -> Scalar {
    if !n.is_power_of_two() {
        return sum_of_powers_slow(x, n);
    }
    if n == 0 || n == 1 {
        return Scalar::from(n as u64);
    }
    let mut m = n;
    let mut result = Scalar::ONE + x;
    let mut factor = *x;
    while m > 2 {
        factor = factor * factor;
        result = result + factor * result;
        m /= 2;
    }
    result
}

/// A straightforward method to compute the sum of powers of `x`.
fn sum_of_powers_slow(x: &Scalar, n: usize) -> Scalar {
    exp_iter(*x).take(n).sum()
}
