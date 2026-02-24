/// Syscall definitions used by `solana_cpi`.
pub use solana_define_syscall::definitions::{
    sol_invoke_signed_c, sol_invoke_signed_rust, sol_set_return_data,
};
use solana_pubkey::Pubkey;

#[deprecated(
    since = "3.1.0",
    note = "Use `solana_define_syscall::definitions::sol_get_return_data` instead"
)]
pub unsafe fn sol_get_return_data(data: *mut u8, length: u64, program_id: *mut Pubkey) -> u64 {
    solana_define_syscall::definitions::sol_get_return_data(data, length, program_id as *mut u8)
}
