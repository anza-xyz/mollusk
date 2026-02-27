use std::hash::{Hash, Hasher};

#[rustfmt::skip]
pub const UNRESERVED_CHAR_MAP: [u8; 256] = [
 // 0     1     2     3     4     5     6     7     8     9     A     B     C     D     E     F
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 0
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 1
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, b'-', b'.',    0, // 2
 b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',    0,    0,    0,    0,    0,    0, // 3
    0, b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', // 4
 b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z',    0,    0,    0,    0, b'_', // 5
    0, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', // 6
 b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z',    0,    0,    0, b'~',    0, // 7
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 8
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 9
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // A
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // B
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // C
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // D
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // E
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // F
];

pub fn get_percent_encoded_value(
    first_digit: Option<u8>,
    second_digit: Option<u8>,
) -> Result<(u8, bool), ()> {
    match (first_digit, second_digit) {
        (Some(first_digit), Some(second_digit)) => {
            let first_digit = hex_digit_to_decimal(first_digit)?;
            let second_digit = hex_digit_to_decimal(second_digit)?;
            let hex_value = first_digit.0 * 16 + second_digit.0;
            let uppercase = first_digit.1 && second_digit.1;
            Ok((hex_value, uppercase))
        }
        _ => Err(()),
    }
}

fn hex_digit_to_decimal(digit: u8) -> Result<(u8, bool), ()> {
    match digit {
        _ if digit >= b'A' && digit <= b'F' => Ok((digit - b'A' + 10, true)),
        _ if digit >= b'a' && digit <= b'f' => Ok((digit - b'a' + 10, false)),
        _ if digit.is_ascii_digit() => Ok((digit - b'0', true)),
        _ => Err(()),
    }
}

/// This function is unsafe because it makes the assumption that the given string is valid ASCII-US.
pub unsafe fn normalize_string(string: &mut String, case_sensitive: bool) {
    let bytes = string.as_mut_vec();
    let mut read_index = 0;
    let mut write_index = 0;

    while read_index < bytes.len() {
        let byte = bytes[read_index];
        read_index += 1;

        if byte == b'%' {
            let first_digit = bytes.get(read_index).cloned();
            let second_digit = bytes.get(read_index + 1).cloned();
            let (hex_value, _) = get_percent_encoded_value(first_digit, second_digit).unwrap();
            read_index += 2;

            if UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                bytes[write_index] = hex_value;
                write_index += 1;
            } else {
                bytes[write_index] = b'%';
                bytes[write_index + 1] = first_digit.unwrap().to_ascii_uppercase();
                bytes[write_index + 2] = second_digit.unwrap().to_ascii_uppercase();
                write_index += 3;
            }
        } else {
            if !case_sensitive {
                bytes[write_index] = byte.to_ascii_lowercase();
            } else {
                bytes[write_index] = byte;
            }

            write_index += 1;
        }
    }

    bytes.truncate(write_index);
}

pub fn percent_encoded_hash<H>(value: &[u8], state: &mut H, case_sensitive: bool)
where
    H: Hasher,
{
    let mut bytes = value.iter();
    let mut length = 0;

    while let Some(byte) = bytes.next() {
        length += 1;

        match byte {
            b'%' => {
                let first_digit = bytes.next().cloned();
                let second_digit = bytes.next().cloned();
                let (hex_value, _) = get_percent_encoded_value(first_digit, second_digit).unwrap();

                if UNRESERVED_CHAR_MAP[hex_value as usize] == 0 {
                    b'%'.hash(state);
                    first_digit.unwrap().to_ascii_uppercase().hash(state);
                    second_digit.unwrap().to_ascii_uppercase().hash(state);
                } else if case_sensitive {
                    hex_value.hash(state);
                } else {
                    hex_value.to_ascii_lowercase().hash(state);
                }
            }
            _ => {
                if case_sensitive {
                    byte.hash(state)
                } else {
                    byte.to_ascii_lowercase().hash(state)
                }
            }
        }
    }

    length.hash(state);
}

fn percent_encoded_equality_helper(
    byte: u8,
    first_digit: Option<u8>,
    second_digit: Option<u8>,
    case_sensitive: bool,
) -> bool {
    if UNRESERVED_CHAR_MAP[byte as usize] == 0 {
        return false;
    }

    match get_percent_encoded_value(first_digit, second_digit) {
        Ok((hex_value, _)) => {
            if case_sensitive {
                hex_value == byte
            } else {
                hex_value.eq_ignore_ascii_case(&byte)
            }
        }
        Err(_) => false,
    }
}

pub fn percent_encoded_equality(left: &[u8], right: &[u8], case_sensitive: bool) -> bool {
    let mut left_bytes = left.iter();
    let mut right_bytes = right.iter();

    loop {
        match (left_bytes.next(), right_bytes.next()) {
            (Some(b'%'), Some(b'%')) => (),
            (Some(b'%'), Some(&right_byte)) => {
                if !percent_encoded_equality_helper(
                    right_byte,
                    left_bytes.next().cloned(),
                    left_bytes.next().cloned(),
                    case_sensitive,
                ) {
                    return false;
                }
            }
            (Some(&left_byte), Some(b'%')) => {
                if !percent_encoded_equality_helper(
                    left_byte,
                    right_bytes.next().cloned(),
                    right_bytes.next().cloned(),
                    case_sensitive,
                ) {
                    return false;
                }
            }
            (Some(left_byte), Some(right_byte)) => {
                let equal = if case_sensitive {
                    left_byte == right_byte
                } else {
                    left_byte.eq_ignore_ascii_case(&right_byte)
                };

                if !equal {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;

    use super::*;

    #[test]
    fn test_equality() {
        // Case sensitive

        assert!(percent_encoded_equality(b"abc", b"abc", true));
        assert!(percent_encoded_equality(b"abc", b"%61bc", true));
        assert!(percent_encoded_equality(b"MNO", b"%4DNO", true));
        assert!(percent_encoded_equality(b"MNO", b"%4dNO", true));

        assert!(!percent_encoded_equality(b"abc", b"xyz", true));
        assert!(!percent_encoded_equality(b"abc", b"Abc", true));
        assert!(!percent_encoded_equality(b"abc", b"%41bc", true));
        assert!(!percent_encoded_equality(b"/", b"%2F", true));

        // Case insensitive

        assert!(percent_encoded_equality(b"abc", b"abc", false));
        assert!(percent_encoded_equality(b"abc", b"ABC", false));
        assert!(percent_encoded_equality(b"MNO", b"%4DNO", false));
        assert!(percent_encoded_equality(b"MNO", b"%4dNO", false));
        assert!(percent_encoded_equality(b"abc", b"%61bc", false));
        assert!(percent_encoded_equality(b"abc", b"%41bc", false));

        assert!(!percent_encoded_equality(b"abc", b"xyz", false));
        assert!(!percent_encoded_equality(b"/", b"%2F", false));
    }

    #[test]
    fn test_hash() {
        fn hash<State>(value: &[u8], state: &State, case_sensitive: bool) -> u64
        where
            State: BuildHasher,
        {
            let mut hasher = state.build_hasher();
            percent_encoded_hash(value, &mut hasher, case_sensitive);
            hasher.finish()
        }

        fn compare_hashes<State>(
            left: &[u8],
            right: &[u8],
            state: &State,
            case_sensitive: bool,
        ) -> bool
        where
            State: BuildHasher,
        {
            let left_hash = hash(left, state, case_sensitive);
            let right_hash = hash(right, state, case_sensitive);
            left_hash == right_hash
        }

        let state = RandomState::new();

        // Case sensitive

        assert!(compare_hashes(b"abc", b"abc", &state, true));
        assert!(compare_hashes(b"abc", b"%61bc", &state, true));
        assert!(compare_hashes(b"MNO", b"%4DNO", &state, true));
        assert!(compare_hashes(b"MNO", b"%4dNO", &state, true));

        assert!(!compare_hashes(b"abc", b"xyz", &state, true));
        assert!(!compare_hashes(b"abc", b"Abc", &state, true));
        assert!(!compare_hashes(b"abc", b"%41bc", &state, true));
        assert!(!compare_hashes(b"/", b"%2F", &state, true));

        // Case insensitive

        assert!(compare_hashes(b"abc", b"abc", &state, false));
        assert!(compare_hashes(b"abc", b"ABC", &state, false));
        assert!(compare_hashes(b"MNO", b"%4DNO", &state, false));
        assert!(compare_hashes(b"MNO", b"%4dNO", &state, false));
        assert!(compare_hashes(b"abc", b"%61bc", &state, false));
        assert!(compare_hashes(b"abc", b"%41bc", &state, false));

        assert!(!compare_hashes(b"abc", b"xyz", &state, false));
        assert!(!compare_hashes(b"/", b"%2F", &state, false));
    }
}
