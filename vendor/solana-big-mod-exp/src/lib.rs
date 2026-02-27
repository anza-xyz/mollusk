#[repr(C)]
pub struct BigModExpParams {
    pub base: *const u8,
    pub base_len: u64,
    pub exponent: *const u8,
    pub exponent_len: u64,
    pub modulus: *const u8,
    pub modulus_len: u64,
}

/// Big integer modular exponentiation
pub fn big_mod_exp(base: &[u8], exponent: &[u8], modulus: &[u8]) -> Vec<u8> {
    #[cfg(not(target_os = "solana"))]
    {
        use {
            num_bigint::BigUint,
            num_traits::{One, Zero},
        };

        let modulus_len = modulus.len();
        let base = BigUint::from_bytes_be(base);
        let exponent = BigUint::from_bytes_be(exponent);
        let modulus = BigUint::from_bytes_be(modulus);

        if modulus.is_zero() || modulus.is_one() {
            return vec![0_u8; modulus_len];
        }

        let ret_int = base.modpow(&exponent, &modulus);
        let ret_int = ret_int.to_bytes_be();
        let mut return_value = vec![0_u8; modulus_len.saturating_sub(ret_int.len())];
        return_value.extend(ret_int);
        return_value
    }

    #[cfg(target_os = "solana")]
    {
        let mut return_value = vec![0_u8; modulus.len()];

        let param = BigModExpParams {
            base: base as *const _ as *const u8,
            base_len: base.len() as u64,
            exponent: exponent as *const _ as *const u8,
            exponent_len: exponent.len() as u64,
            modulus: modulus as *const _ as *const u8,
            modulus_len: modulus.len() as u64,
        };
        unsafe {
            solana_define_syscall::definitions::sol_big_mod_exp(
                &param as *const _ as *const u8,
                return_value.as_mut_slice() as *mut _ as *mut u8,
            )
        };

        return_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_mod_exp_test() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct TestCase {
            base: String,
            exponent: String,
            modulus: String,
            expected: String,
        }

        let test_data = include_str!("../tests/data/big_mod_exp_cases.json");

        let test_cases: Vec<TestCase> = serde_json::from_str(test_data).unwrap();
        test_cases.iter().for_each(|test| {
            let base = array_bytes::hex2bytes_unchecked(&test.base);
            let exponent = array_bytes::hex2bytes_unchecked(&test.exponent);
            let modulus = array_bytes::hex2bytes_unchecked(&test.modulus);
            let expected = array_bytes::hex2bytes_unchecked(&test.expected);
            let result = big_mod_exp(base.as_slice(), exponent.as_slice(), modulus.as_slice());
            assert_eq!(result, expected);
        });
    }
}
