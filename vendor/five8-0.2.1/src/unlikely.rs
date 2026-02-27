#[inline(always)]
#[cold]
fn cold() {}

#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    if b {
        cold()
    }
    b
}
