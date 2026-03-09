use {
    crate::{
        authorized_voters::AuthorizedVoters,
        state::{
            BlockTimestamp, LandedVote, Lockout, VoteStateV3, VoteStateV4,
            BLS_PUBLIC_KEY_COMPRESSED_SIZE, MAX_EPOCH_CREDITS_HISTORY, MAX_ITEMS,
            MAX_LOCKOUT_HISTORY,
        },
    },
    solana_clock::Epoch,
    solana_instruction_error::InstructionError,
    solana_pubkey::Pubkey,
    solana_serialize_utils::cursor::{
        read_bool, read_i64, read_option_u64, read_pubkey, read_pubkey_into, read_u16, read_u32,
        read_u64, read_u8,
    },
    std::{
        collections::VecDeque,
        io::{BufRead, Cursor, Read},
        ptr::addr_of_mut,
    },
};

// This is to reset vote_state to T::default() if deserialize fails or panics.
struct DropGuard<T: Default> {
    vote_state: *mut T,
}

impl<T: Default> Drop for DropGuard<T> {
    fn drop(&mut self) {
        // Safety:
        //
        // Deserialize failed or panicked so at this point vote_state is uninitialized. We
        // must write a new _valid_ value into it or after returning (or unwinding) from
        // this function the caller is left with an uninitialized `&mut T`, which is UB
        // (references must always be valid).
        //
        // This is always safe and doesn't leak memory because deserialize_into_ptr() writes
        // into the fields that heap alloc only when it returns Ok().
        unsafe {
            self.vote_state.write(T::default());
        }
    }
}

pub(crate) fn deserialize_into<T: Default>(
    input: &[u8],
    vote_state: &mut T,
    deserialize_fn: impl FnOnce(&[u8], *mut T) -> Result<(), InstructionError>,
) -> Result<(), InstructionError> {
    // Rebind vote_state to *mut T so that the &mut binding isn't accessible
    // anymore, preventing accidental use after this point.
    //
    // NOTE: switch to ptr::from_mut() once platform-tools moves to rustc >= 1.76
    let vote_state = vote_state as *mut T;

    // Safety: vote_state is valid to_drop (see drop_in_place() docs). After
    // dropping, the pointer is treated as uninitialized and only accessed
    // through ptr::write, which is safe as per drop_in_place docs.
    unsafe {
        std::ptr::drop_in_place(vote_state);
    }

    // This is to reset vote_state to T::default() if deserialize fails or panics.
    let guard = DropGuard { vote_state };

    let res = deserialize_fn(input, vote_state);
    if res.is_ok() {
        std::mem::forget(guard);
    }

    res
}

pub(crate) fn deserialize_vote_state_into_v1_14_11(
    cursor: &mut Cursor<&[u8]>,
    vote_state: *mut crate::state::vote_state_1_14_11::VoteState1_14_11,
) -> Result<(), InstructionError> {
    // General safety note: we must use addr_of_mut! to access the `vote_state` fields as the value
    // is assumed to be _uninitialized_, so creating references to the state or any of its inner
    // fields is UB.

    read_pubkey_into(
        cursor,
        // Safety: if vote_state is non-null, node_pubkey is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).node_pubkey) },
    )?;
    read_pubkey_into(
        cursor,
        // Safety: if vote_state is non-null, authorized_withdrawer is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).authorized_withdrawer) },
    )?;
    let commission = read_u8(cursor)?;
    let votes = read_votes_as_lockouts(cursor)?; // `Vec<Lockout>`
    let root_slot = read_option_u64(cursor)?;
    let authorized_voters = read_authorized_voters(cursor)?;

    read_prior_voters_into(
        cursor,
        // Safety: if vote_state is non-null, prior_voters is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).prior_voters) },
    )?;

    let epoch_credits = read_epoch_credits(cursor)?;
    let last_timestamp = read_last_timestamp(cursor)?;

    // Safety: if vote_state is non-null, all the fields are guaranteed to be
    // valid pointers.
    //
    // Heap allocated collections - votes, authorized_voters, prior_voters, and epoch_credits -
    // are guaranteed not to leak after this point as the VoteState1_14_11 is fully
    // initialized and will be regularly dropped.
    unsafe {
        addr_of_mut!((*vote_state).commission).write(commission);
        addr_of_mut!((*vote_state).votes).write(votes);
        addr_of_mut!((*vote_state).root_slot).write(root_slot);
        addr_of_mut!((*vote_state).authorized_voters).write(authorized_voters);
        addr_of_mut!((*vote_state).epoch_credits).write(epoch_credits);
        addr_of_mut!((*vote_state).last_timestamp).write(last_timestamp);
    }

    Ok(())
}

pub(super) fn deserialize_vote_state_into_v3(
    cursor: &mut Cursor<&[u8]>,
    vote_state: *mut VoteStateV3,
    has_latency: bool,
) -> Result<(), InstructionError> {
    // General safety note: we must use add_or_mut! to access the `vote_state` fields as the value
    // is assumed to be _uninitialized_, so creating references to the state or any of its inner
    // fields is UB.

    read_pubkey_into(
        cursor,
        // Safety: if vote_state is non-null, node_pubkey is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).node_pubkey) },
    )?;
    read_pubkey_into(
        cursor,
        // Safety: if vote_state is non-null, authorized_withdrawer is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).authorized_withdrawer) },
    )?;
    let commission = read_u8(cursor)?;
    let votes = read_votes(cursor, has_latency)?;
    let root_slot = read_option_u64(cursor)?;
    let authorized_voters = read_authorized_voters(cursor)?;

    read_prior_voters_into(
        cursor,
        // Safety: if vote_state is non-null, prior_voters is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).prior_voters) },
    )?;

    let epoch_credits = read_epoch_credits(cursor)?;
    let last_timestamp = read_last_timestamp(cursor)?;

    // Safety: if vote_state is non-null, all the fields are guaranteed to be
    // valid pointers.
    //
    // Heap allocated collections - votes, authorized_voters and epoch_credits -
    // are guaranteed not to leak after this point as the VoteStateV3 is fully
    // initialized and will be regularly dropped.
    unsafe {
        addr_of_mut!((*vote_state).commission).write(commission);
        addr_of_mut!((*vote_state).votes).write(votes);
        addr_of_mut!((*vote_state).root_slot).write(root_slot);
        addr_of_mut!((*vote_state).authorized_voters).write(authorized_voters);
        addr_of_mut!((*vote_state).epoch_credits).write(epoch_credits);
        addr_of_mut!((*vote_state).last_timestamp).write(last_timestamp);
    }

    Ok(())
}

#[derive(PartialEq)]
pub(crate) enum SourceVersion<'a> {
    V1_14_11 { vote_pubkey: &'a Pubkey },
    V3 { vote_pubkey: &'a Pubkey },
    V4,
}

pub(crate) fn deserialize_vote_state_into_v4<'a>(
    cursor: &mut Cursor<&[u8]>,
    vote_state: *mut VoteStateV4,
    source_version: SourceVersion<'a>,
) -> Result<(), InstructionError> {
    // General safety note: we must use addr_of_mut! to access the `vote_state` fields as the value
    // is assumed to be _uninitialized_, so creating references to the state or any of its inner
    // fields is UB.

    // Read common fields that are in the same position for all versions.
    // Keep the node pubkey value around for later fields.
    let node_pubkey = read_pubkey(cursor)?;
    unsafe {
        // Safety: if vote_state is non-null, node_pubkey is guaranteed to be valid too
        addr_of_mut!((*vote_state).node_pubkey).write(node_pubkey);
    }
    read_pubkey_into(
        cursor,
        // Safety: if vote_state is non-null, authorized_withdrawer is guaranteed to be valid too
        unsafe { addr_of_mut!((*vote_state).authorized_withdrawer) },
    )?;

    // Handle version-specific fields and conversions.
    let (
        inflation_rewards_commission_bps,
        block_revenue_commission_bps,
        pending_delegator_rewards,
        bls_pubkey_compressed,
    ) = match source_version {
        SourceVersion::V4 => {
            // V4 has collectors and commission fields here.
            read_pubkey_into(cursor, unsafe {
                addr_of_mut!((*vote_state).inflation_rewards_collector)
            })?;
            read_pubkey_into(cursor, unsafe {
                addr_of_mut!((*vote_state).block_revenue_collector)
            })?;

            // Read the basis points and pending rewards directly.
            let inflation = read_u16(cursor)?;
            let block = read_u16(cursor)?;
            let pending = read_u64(cursor)?;

            // Read the BLS pubkey.
            let bls_pubkey_compressed = read_option_bls_public_key_compressed(cursor)?;

            (inflation, block, pending, bls_pubkey_compressed)
        }
        SourceVersion::V1_14_11 { vote_pubkey } | SourceVersion::V3 { vote_pubkey } => {
            // V1_14_11 and V3 have commission field here.
            let commission = read_u8(cursor)?;

            // Set collectors based on SIMD-0185.
            // Safety: if vote_state is non-null, collectors are guaranteed to be valid too
            unsafe {
                addr_of_mut!((*vote_state).inflation_rewards_collector).write(*vote_pubkey);
                addr_of_mut!((*vote_state).block_revenue_collector).write(node_pubkey);
            }

            // Convert commission to basis points and set block revenue to 100%.
            // No rewards tracked. No BLS pubkey.
            (
                u16::from(commission).saturating_mul(100),
                10_000u16,
                0u64,
                None,
            )
        }
    };

    // For V3 and V4, `has_latency` is always true.
    let has_latency = !matches!(source_version, SourceVersion::V1_14_11 { .. });
    let votes = read_votes(cursor, has_latency)?;
    let root_slot = read_option_u64(cursor)?;
    let authorized_voters = read_authorized_voters(cursor)?;

    // V1_14_11 and V3 have `prior_voters` field here.
    // Skip, since V4 doesn't have this field.
    if !matches!(source_version, SourceVersion::V4) {
        skip_prior_voters(cursor)?;
    }

    let epoch_credits = read_epoch_credits(cursor)?;
    let last_timestamp = read_last_timestamp(cursor)?;

    // Safety: if vote_state is non-null, all the fields are guaranteed to be
    // valid pointers.
    //
    // Heap allocated collections - votes, authorized_voters and epoch_credits -
    // are guaranteed not to leak after this point as the VoteStateV4 is fully
    // initialized and will be regularly dropped.
    unsafe {
        addr_of_mut!((*vote_state).inflation_rewards_commission_bps)
            .write(inflation_rewards_commission_bps);
        addr_of_mut!((*vote_state).block_revenue_commission_bps)
            .write(block_revenue_commission_bps);
        addr_of_mut!((*vote_state).pending_delegator_rewards).write(pending_delegator_rewards);
        addr_of_mut!((*vote_state).bls_pubkey_compressed).write(bls_pubkey_compressed);
        addr_of_mut!((*vote_state).votes).write(votes);
        addr_of_mut!((*vote_state).root_slot).write(root_slot);
        addr_of_mut!((*vote_state).authorized_voters).write(authorized_voters);
        addr_of_mut!((*vote_state).epoch_credits).write(epoch_credits);
        addr_of_mut!((*vote_state).last_timestamp).write(last_timestamp);
    }

    Ok(())
}

fn read_votes<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
    has_latency: bool,
) -> Result<VecDeque<LandedVote>, InstructionError> {
    let vote_count = read_u64(cursor)? as usize;
    let mut votes = VecDeque::with_capacity(vote_count.min(MAX_LOCKOUT_HISTORY));

    for _ in 0..vote_count {
        let latency = if has_latency { read_u8(cursor)? } else { 0 };

        let slot = read_u64(cursor)?;
        let confirmation_count = read_u32(cursor)?;
        let lockout = Lockout::new_with_confirmation_count(slot, confirmation_count);

        votes.push_back(LandedVote { latency, lockout });
    }

    Ok(votes)
}

fn read_votes_as_lockouts<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
) -> Result<VecDeque<Lockout>, InstructionError> {
    let vote_count = read_u64(cursor)? as usize;
    let mut votes = VecDeque::with_capacity(vote_count.min(MAX_LOCKOUT_HISTORY));

    for _ in 0..vote_count {
        let slot = read_u64(cursor)?;
        let confirmation_count = read_u32(cursor)?;
        let lockout = Lockout::new_with_confirmation_count(slot, confirmation_count);

        votes.push_back(lockout);
    }

    Ok(votes)
}

fn read_authorized_voters<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
) -> Result<AuthorizedVoters, InstructionError> {
    let authorized_voter_count = read_u64(cursor)?;
    let mut authorized_voters = AuthorizedVoters::default();

    for _ in 0..authorized_voter_count {
        let epoch = read_u64(cursor)?;
        let authorized_voter = read_pubkey(cursor)?;
        authorized_voters.insert(epoch, authorized_voter);
    }

    Ok(authorized_voters)
}

fn read_prior_voters_into<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
    prior_voters: *mut crate::state::CircBuf<(Pubkey, Epoch, Epoch)>,
) -> Result<(), InstructionError> {
    // Safety: if prior_voters is non-null, its buf field is guaranteed to be valid too
    unsafe {
        let prior_voters_buf = addr_of_mut!((*prior_voters).buf) as *mut (Pubkey, Epoch, Epoch);

        for i in 0..MAX_ITEMS {
            let prior_voter = read_pubkey(cursor)?;
            let from_epoch = read_u64(cursor)?;
            let until_epoch = read_u64(cursor)?;

            prior_voters_buf
                .add(i)
                .write((prior_voter, from_epoch, until_epoch));
        }

        (*prior_voters).idx = read_u64(cursor)? as usize;
        (*prior_voters).is_empty = read_bool(cursor)?;
    }
    Ok(())
}

// Same navigation as `read_prior_voters_into`, but does not perform any writes.
// Merely updates the cursor to skip over this section.
fn skip_prior_voters<T: AsRef<[u8]>>(cursor: &mut Cursor<T>) -> Result<(), InstructionError> {
    const PRIOR_VOTERS_SIZE: usize = MAX_ITEMS * core::mem::size_of::<(Pubkey, Epoch, Epoch)>() +
            core::mem::size_of::<u64>() /* idx */ +
            core::mem::size_of::<bool>() /* is_empty */;

    cursor.consume(PRIOR_VOTERS_SIZE);

    let bytes = cursor.get_ref().as_ref();
    if cursor.position() as usize > bytes.len() {
        return Err(InstructionError::InvalidAccountData);
    }

    Ok(())
}

fn read_option_bls_public_key_compressed<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
) -> Result<Option<[u8; BLS_PUBLIC_KEY_COMPRESSED_SIZE]>, InstructionError> {
    let variant = read_u8(cursor)?;
    match variant {
        0 => Ok(None),
        1 => {
            let mut buf = [0; BLS_PUBLIC_KEY_COMPRESSED_SIZE];
            cursor
                .read_exact(&mut buf)
                .map_err(|_| InstructionError::InvalidAccountData)?;
            Ok(Some(buf))
        }
        _ => Err(InstructionError::InvalidAccountData),
    }
}

fn read_epoch_credits<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
) -> Result<Vec<(Epoch, u64, u64)>, InstructionError> {
    let epoch_credit_count = read_u64(cursor)? as usize;
    let mut epoch_credits = Vec::with_capacity(epoch_credit_count.min(MAX_EPOCH_CREDITS_HISTORY));

    for _ in 0..epoch_credit_count {
        let epoch = read_u64(cursor)?;
        let credits = read_u64(cursor)?;
        let prev_credits = read_u64(cursor)?;
        epoch_credits.push((epoch, credits, prev_credits));
    }

    Ok(epoch_credits)
}

fn read_last_timestamp<T: AsRef<[u8]>>(
    cursor: &mut Cursor<T>,
) -> Result<BlockTimestamp, InstructionError> {
    let slot = read_u64(cursor)?;
    let timestamp = read_i64(cursor)?;

    Ok(BlockTimestamp { slot, timestamp })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIOR_VOTERS_SIZE: usize = MAX_ITEMS * core::mem::size_of::<(Pubkey, Epoch, Epoch)>() +
        core::mem::size_of::<u64>() /* idx */ +
        core::mem::size_of::<bool>() /* is_empty */;

    #[test]
    fn test_skip_prior_voters_success() {
        // Correct size.
        let buffer = vec![0u8; PRIOR_VOTERS_SIZE];
        let mut cursor = Cursor::new(&buffer[..]);

        // Should succeed.
        let result = skip_prior_voters(&mut cursor);
        assert!(result.is_ok());

        // Cursor should be at the end.
        assert_eq!(cursor.position() as usize, PRIOR_VOTERS_SIZE);
    }

    #[test]
    fn test_skip_prior_voters_success_with_offset() {
        // We'll use an offset of 100 and create a buffer with 100 extra bytes.
        let offset = 100;
        let buffer = vec![0u8; PRIOR_VOTERS_SIZE + offset];
        let mut cursor = Cursor::new(&buffer[..]);

        // Move cursor to offset position.
        cursor.set_position(offset as u64);

        // Should succeed.
        let result = skip_prior_voters(&mut cursor);
        assert!(result.is_ok());

        // Cursor should be at the end.
        assert_eq!(cursor.position() as usize, PRIOR_VOTERS_SIZE + offset);
    }

    #[test]
    fn test_skip_prior_voters_buffer_too_small() {
        // Too small.
        let buffer = vec![0u8; PRIOR_VOTERS_SIZE - 1];
        let mut cursor = Cursor::new(&buffer[..]);

        // Should fail.
        let result = skip_prior_voters(&mut cursor);
        assert_eq!(result, Err(InstructionError::InvalidAccountData));
    }

    #[test]
    fn test_skip_prior_voters_insufficient_remaining() {
        // Create a buffer with 100 extra bytes.
        let buffer = vec![0u8; PRIOR_VOTERS_SIZE + 100];
        let mut cursor = Cursor::new(&buffer[..]);

        // Position cursor so there's not enough remaining bytes.
        cursor.set_position(101);

        // Should fail because cursor position > bytes length.
        let result = skip_prior_voters(&mut cursor);
        assert_eq!(result, Err(InstructionError::InvalidAccountData));
    }
}
