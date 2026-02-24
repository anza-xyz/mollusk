#[inline(always)]
pub(crate) fn fd_ulong_find_lsb_w_default(x: u64, d: i32) -> i32 {
    #[cfg(target_arch = "x86_64")]
    {
        union IorU64 {
            i: i64,
            u: u64,
        }
        let mut r = IorU64 { u: x };
        let c = IorU64 { i: d as i64 };
        // see https://doc.rust-lang.org/nightly/rust-by-example/unsafe/asm.html#labels
        // on why we use options(att_syntax).
        // The non-bmi1 code didn't work before adding that.
        #[cfg(target_feature = "bmi1")]
        unsafe {
            core::arch::asm!(
                "tzcnt {0}, {0}",
                "cmovb {1}, {0}",
                inout(reg) r.u,
                in(reg) c.u,
                options(att_syntax)
            )
        };
        #[cfg(not(target_feature = "bmi1"))]
        unsafe {
            core::arch::asm!(
                "bsf {0}, {0}",
                "cmovz {1}, {0}",
                inout(reg) r.u,
                in(reg) c.u,
                options(att_syntax)
            )
        };
        unsafe { r.i as i32 }
    }
    #[cfg(not(target_arch = "x86_64"))]
    if x == 0 {
        d
    } else {
        x.trailing_zeros() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_ulong_find_lsb_w_default() {
        assert_eq!(fd_ulong_find_lsb_w_default(0, 64), 64);
        assert_eq!(fd_ulong_find_lsb_w_default(1, 64), 0);
        assert_eq!(fd_ulong_find_lsb_w_default(4, 64), 2);
    }
}
