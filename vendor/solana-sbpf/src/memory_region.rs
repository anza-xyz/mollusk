//! This module defines memory regions

use crate::{
    aligned_memory::Pod,
    ebpf,
    error::{EbpfError, ProgramResult},
    program::SBPFVersion,
    vm::Config,
};
use std::{array, cell::UnsafeCell, fmt, mem, ops::Range, ptr};

/* Explanation of the Gapped Memory

    The MemoryMapping supports a special mapping mode which is used for the stack MemoryRegion.
    In this mode the backing address space of the host is sliced in power-of-two aligned frames.
    The exponent of this alignment is specified in vm_gap_shift. Then the virtual address space
    of the guest is spread out in a way which leaves gaps, the same size as the frames, in
    between the frames. This effectively doubles the size of the guests virtual address space.
    But the actual mapped memory stays the same, as the gaps are not mapped and accessing them
    results in an AccessViolation.

    Guest: frame 0 | gap 0 | frame 1 | gap 1 | frame 2 | gap 2 | ...
              |                /                 /
              |          *----*    *------------*
              |         /         /
    Host:  frame 0 | frame 1 | frame 2 | ...
*/

/// Callback executed before generate_access_violation()
pub type AccessViolationHandler = Box<dyn Fn(&mut MemoryRegion, u64, AccessType, u64, u64)>;
/// Fail always
#[allow(clippy::result_unit_err)]
pub fn default_access_violation_handler(
    _region: &mut MemoryRegion,
    _region_max_len: u64,
    _access_type: AccessType,
    _vm_addr: u64,
    _len: u64,
) {
}

/// Memory region for bounds checking and address translation
#[derive(Default, Eq, PartialEq, Clone)]
#[repr(C, align(32))]
pub struct MemoryRegion {
    /// start host address
    pub host_addr: u64,
    /// start virtual address
    pub vm_addr: u64,
    /// Length in bytes
    pub len: u64,
    /// Size of regular gaps as bit shift (63 means this region is continuous)
    pub vm_gap_shift: u8,
    /// Is `AccessType::Store` allowed without triggering an access violation
    pub writable: bool,
    /// User defined payload for the [AccessViolationHandler]
    pub access_violation_handler_payload: Option<u16>,
}

impl MemoryRegion {
    fn new(slice: &[u8], vm_addr: u64, vm_gap_size: u64, writable: bool) -> Self {
        let mut vm_gap_shift = (std::mem::size_of::<u64>() as u8)
            .saturating_mul(8)
            .saturating_sub(1);
        if vm_gap_size > 0 {
            vm_gap_shift = vm_gap_shift.saturating_sub(vm_gap_size.leading_zeros() as u8);
            debug_assert_eq!(Some(vm_gap_size), 1_u64.checked_shl(vm_gap_shift as u32));
        };
        MemoryRegion {
            host_addr: slice.as_ptr() as u64,
            vm_addr,
            len: slice.len() as u64,
            vm_gap_shift,
            writable,
            access_violation_handler_payload: None,
        }
    }

    /// Only to be used in tests and benches
    pub fn new_for_testing(slice: &[u8], vm_addr: u64, vm_gap_size: u64, writable: bool) -> Self {
        Self::new(slice, vm_addr, vm_gap_size, writable)
    }

    /// Creates a new readonly MemoryRegion from a slice
    pub fn new_readonly(slice: &[u8], vm_addr: u64) -> Self {
        Self::new(slice, vm_addr, 0, false)
    }

    /// Creates a new writable MemoryRegion from a mutable slice
    pub fn new_writable(slice: &mut [u8], vm_addr: u64) -> Self {
        Self::new(&*slice, vm_addr, 0, true)
    }

    /// Creates a new writable gapped MemoryRegion from a mutable slice
    pub fn new_writable_gapped(slice: &mut [u8], vm_addr: u64, vm_gap_size: u64) -> Self {
        Self::new(&*slice, vm_addr, vm_gap_size, true)
    }

    /// Returns the vm address space covered by this MemoryRegion
    pub fn vm_addr_range(&self) -> Range<u64> {
        if self.vm_gap_shift == 63 {
            self.vm_addr..self.vm_addr.saturating_add(self.len)
        } else {
            self.vm_addr..self.vm_addr.saturating_add(self.len.saturating_mul(2))
        }
    }

    /// Convert a virtual machine address into a host address
    pub fn vm_to_host(&self, access_type: AccessType, vm_addr: u64, len: u64) -> Option<u64> {
        if access_type == AccessType::Store && !self.writable {
            return None;
        }

        // This can happen if a region starts at an offset from the base region
        // address, eg with rodata regions if config.optimize_rodata = true, see
        // Elf::get_ro_region.
        if vm_addr < self.vm_addr {
            return None;
        }

        let begin_offset = vm_addr.saturating_sub(self.vm_addr);
        let is_in_gap = (begin_offset
            .checked_shr(self.vm_gap_shift as u32)
            .unwrap_or(0)
            & 1)
            == 1;
        let gap_mask = (-1i64).checked_shl(self.vm_gap_shift as u32).unwrap_or(0) as u64;
        let gapped_offset =
            (begin_offset & gap_mask).checked_shr(1).unwrap_or(0) | (begin_offset & !gap_mask);
        if let Some(end_offset) = gapped_offset.checked_add(len) {
            if end_offset <= self.len && !is_in_gap {
                return Some(self.host_addr.saturating_add(gapped_offset));
            }
        }
        None
    }
}

impl fmt::Debug for MemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "host_addr: {:#x?}-{:#x?}, vm_addr: {:#x?}-{:#x?}, len: {}, writable: {}, payload {:?}",
            self.host_addr,
            self.host_addr.saturating_add(self.len),
            self.vm_addr,
            self.vm_addr_range().end,
            self.len,
            self.writable,
            self.access_violation_handler_payload,
        )
    }
}
impl std::cmp::PartialOrd for MemoryRegion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for MemoryRegion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.vm_addr.cmp(&other.vm_addr)
    }
}

/// Type of memory access
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessType {
    /// Read
    Load,
    /// Write
    Store,
}

/// Common parts of [UnalignedMemoryMapping] and [AlignedMemoryMapping]
pub struct CommonMemoryMapping<'a> {
    /// Mapped memory regions
    regions: Box<[MemoryRegion]>,
    /// Access violation handler
    access_violation_handler: AccessViolationHandler,
    /// VM configuration
    config: &'a Config,
    /// Executable sbpf_version
    sbpf_version: SBPFVersion,
}

impl CommonMemoryMapping<'_> {
    fn generate_access_violation(
        &self,
        access_type: AccessType,
        vm_addr: u64,
        len: u64,
    ) -> ProgramResult {
        let stack_frame = (vm_addr as i64)
            .saturating_sub(ebpf::MM_STACK_START as i64)
            .checked_div(self.config.stack_frame_size as i64)
            .unwrap_or(0);
        if !self.sbpf_version.dynamic_stack_frames()
            && (-1..(self.config.max_call_depth as i64).saturating_add(1)).contains(&stack_frame)
        {
            ProgramResult::Err(EbpfError::StackAccessViolation(
                access_type,
                vm_addr,
                len,
                stack_frame,
            ))
        } else {
            let region_name = match vm_addr & (!ebpf::MM_RODATA_START.saturating_sub(1)) {
                ebpf::MM_RODATA_START => "program",
                ebpf::MM_STACK_START => "stack",
                ebpf::MM_HEAP_START => "heap",
                ebpf::MM_INPUT_START => "input",
                _ => "unknown",
            };
            ProgramResult::Err(EbpfError::AccessViolation(
                access_type,
                vm_addr,
                len,
                region_name,
            ))
        }
    }
}

/// Memory mapping based on eytzinger search.
pub struct UnalignedMemoryMapping<'a> {
    /// Common parts
    common: CommonMemoryMapping<'a>,
    /// Regions vm_addr fields in Eytzinger order
    region_addresses: Box<[u64]>,
    /// Converts the Eytzinger order back to the original order
    region_index_lookup: Box<[usize]>,
    /// Cache of the last `MappingCache::SIZE` vm_addr => region_index lookups
    cache: UnsafeCell<MappingCache>,
}

impl fmt::Debug for UnalignedMemoryMapping<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnalignedMemoryMapping")
            .field("regions", &self.common.regions)
            .field("cache", &self.cache)
            .finish()
    }
}

impl<'a> UnalignedMemoryMapping<'a> {
    fn construct_eytzinger_order(&mut self, mut in_index: usize, out_index: usize) -> usize {
        if out_index >= self.common.regions.len() {
            return in_index;
        }
        in_index =
            self.construct_eytzinger_order(in_index, out_index.saturating_mul(2).saturating_add(1));
        self.region_addresses[out_index] = self.common.regions[in_index].vm_addr;
        self.region_index_lookup[out_index] = in_index;
        self.construct_eytzinger_order(
            in_index.saturating_add(1),
            out_index.saturating_mul(2).saturating_add(2),
        )
    }

    /// Creates a new MemoryMapping structure from the given regions
    pub fn new_with_access_violation_handler(
        mut regions: Vec<MemoryRegion>,
        config: &'a Config,
        sbpf_version: SBPFVersion,
        access_violation_handler: AccessViolationHandler,
    ) -> Result<Self, EbpfError> {
        debug_assert!(
            sbpf_version <= SBPFVersion::V3,
            "SBPFv4 and later versions do not support unaligned memory"
        );
        regions.sort();
        let number_of_regions = regions.len();
        for index in 1..number_of_regions {
            let first = &regions[index.saturating_sub(1)];
            let second = &regions[index];
            if first.vm_addr_range().end > second.vm_addr {
                return Err(EbpfError::InvalidMemoryRegion(index));
            }
        }
        let mut result = Self {
            common: CommonMemoryMapping {
                regions: regions.into_boxed_slice(),
                access_violation_handler,
                config,
                sbpf_version,
            },
            region_addresses: vec![0; number_of_regions].into_boxed_slice(),
            region_index_lookup: vec![0; number_of_regions].into_boxed_slice(),
            cache: UnsafeCell::new(MappingCache::new()),
        };
        result.construct_eytzinger_order(0, 0);
        Ok(result)
    }

    /// Returns the `MemoryRegion` which may contain the given address.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn find_region(&self, vm_addr: u64) -> Option<(usize, &MemoryRegion)> {
        // Safety:
        // &mut references to the mapping cache are only created internally from methods that do not
        // invoke each other. UnalignedMemoryMapping is !Sync, so the cache reference below is
        // guaranteed to be unique.
        let cache = unsafe { &mut *self.cache.get() };
        if let Some(index) = cache.find(vm_addr) {
            // Safety:
            // Cached index, we validated it before caching it. See the corresponding safety section
            // in the miss branch.
            Some((index, unsafe { self.common.regions.get_unchecked(index) }))
        } else {
            let mut index = 1;
            while index <= self.region_addresses.len() {
                // Safety:
                // we start the search at index=1 and in the loop condition check
                // for index <= len, so bound checks can be avoided
                index = (index << 1)
                    + unsafe { *self.region_addresses.get_unchecked(index - 1) <= vm_addr }
                        as usize;
            }
            index >>= index.trailing_zeros() + 1;
            if index == 0 {
                return None;
            }
            // Safety:
            // we check for index==0 above, and by construction if we get here index
            // must be contained in region
            index = unsafe { *self.region_index_lookup.get_unchecked(index - 1) };
            let region = unsafe { self.common.regions.get_unchecked(index) };
            cache.insert(region.vm_addr_range(), index);
            Some((index, region))
        }
    }

    /// Replaces the `MemoryRegion` at the given index
    pub fn replace_region(&mut self, index: usize, region: MemoryRegion) -> Result<(), EbpfError> {
        self.common.regions[index] = region;
        self.cache.get_mut().flush();
        Ok(())
    }
}

/// Memory mapping that uses the upper half of an address to identify the
/// underlying memory region.
pub struct AlignedMemoryMapping<'a> {
    /// Common parts
    common: CommonMemoryMapping<'a>,
}

impl fmt::Debug for AlignedMemoryMapping<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlignedMemoryMapping")
            .field("regions", &self.common.regions)
            .finish()
    }
}

impl<'a> AlignedMemoryMapping<'a> {
    /// Creates a new MemoryMapping structure from the given regions
    pub fn new_with_access_violation_handler(
        mut regions: Vec<MemoryRegion>,
        config: &'a Config,
        sbpf_version: SBPFVersion,
        access_violation_handler: AccessViolationHandler,
    ) -> Result<Self, EbpfError> {
        regions.insert(0, MemoryRegion::new_readonly(&[], 0));
        regions.sort();
        for (index, region) in regions.iter().enumerate() {
            if region
                .vm_addr
                .checked_shr(ebpf::VIRTUAL_ADDRESS_BITS as u32)
                .unwrap_or(0)
                != index as u64
            {
                return Err(EbpfError::InvalidMemoryRegion(index));
            }
        }
        Ok(Self {
            common: CommonMemoryMapping {
                regions: regions.into_boxed_slice(),
                access_violation_handler,
                config,
                sbpf_version,
            },
        })
    }

    /// Returns the `MemoryRegion` which may contain the given address.
    #[inline]
    pub fn find_region(&self, vm_addr: u64) -> Option<(usize, &MemoryRegion)> {
        let index = vm_addr.wrapping_shr(ebpf::VIRTUAL_ADDRESS_BITS as u32) as usize;
        if (1..self.common.regions.len()).contains(&index) {
            // Safety: bounds check above
            let region = unsafe { self.common.regions.get_unchecked(index) };
            return Some((index, region));
        }
        None
    }

    /// Replaces the `MemoryRegion` at the given index
    pub fn replace_region(&mut self, index: usize, region: MemoryRegion) -> Result<(), EbpfError> {
        let begin_index = region
            .vm_addr
            .checked_shr(ebpf::VIRTUAL_ADDRESS_BITS as u32)
            .unwrap_or(0) as usize;
        let end_index = region
            .vm_addr
            .saturating_add(region.len.saturating_sub(1))
            .checked_shr(ebpf::VIRTUAL_ADDRESS_BITS as u32)
            .unwrap_or(0) as usize;
        if begin_index != index || end_index != index {
            return Err(EbpfError::InvalidMemoryRegion(index));
        }
        self.common.regions[index] = region;
        Ok(())
    }
}

/// Maps virtual memory to host memory.
#[derive(Debug)]
pub enum MemoryMapping<'a> {
    /// Used when address translation is disabled
    Identity,
    /// Aligned memory mapping which uses the upper half of an address to
    /// identify the underlying memory region.
    Aligned(AlignedMemoryMapping<'a>),
    /// Memory mapping that allows mapping unaligned memory regions.
    Unaligned(UnalignedMemoryMapping<'a>),
}

impl<'a> MemoryMapping<'a> {
    pub(crate) fn new_identity() -> Self {
        MemoryMapping::Identity
    }

    /// Creates a new memory mapping.
    ///
    /// Uses aligned or unaligned memory mapping depending on the value of
    /// `config.aligned_memory_mapping=true`.
    pub fn new_with_access_violation_handler(
        regions: Vec<MemoryRegion>,
        config: &'a Config,
        sbpf_version: SBPFVersion,
        access_violation_handler: AccessViolationHandler,
    ) -> Result<Self, EbpfError> {
        if sbpf_version == SBPFVersion::V4 || config.aligned_memory_mapping {
            AlignedMemoryMapping::new_with_access_violation_handler(
                regions,
                config,
                sbpf_version,
                access_violation_handler,
            )
            .map(MemoryMapping::Aligned)
        } else {
            UnalignedMemoryMapping::new_with_access_violation_handler(
                regions,
                config,
                sbpf_version,
                access_violation_handler,
            )
            .map(MemoryMapping::Unaligned)
        }
    }

    /// Creates a new memory mapping for tests and benches.
    ///
    /// `access_violation_handler` defaults to a function which always returns an error.
    pub fn new(
        regions: Vec<MemoryRegion>,
        config: &'a Config,
        sbpf_version: SBPFVersion,
    ) -> Result<Self, EbpfError> {
        Self::new_with_access_violation_handler(
            regions,
            config,
            sbpf_version,
            Box::new(default_access_violation_handler),
        )
    }

    /// Map virtual memory to host memory.
    pub fn map(&self, access_type: AccessType, vm_addr: u64, len: u64) -> ProgramResult {
        if let Some((_index, region)) = self.find_region(vm_addr) {
            if let Some(host_addr) = region.vm_to_host(access_type, vm_addr, len) {
                return ProgramResult::Ok(host_addr);
            }
        }
        let common = match &self {
            MemoryMapping::Identity => return ProgramResult::Ok(vm_addr),
            MemoryMapping::Aligned(m) => &m.common,
            MemoryMapping::Unaligned(m) => &m.common,
        };
        common.generate_access_violation(access_type, vm_addr, len)
    }

    /// Map virtual memory to host memory and potentially call the [AccessViolationHandler].
    ///
    /// This requires the [MemoryMapping] to be mutable and
    /// can cause previously translated addresses to become invalid.
    #[inline]
    pub fn map_with_access_violation_handler(
        &mut self,
        access_type: AccessType,
        vm_addr: u64,
        len: u64,
    ) -> ProgramResult {
        let common = match &self {
            MemoryMapping::Identity => return ProgramResult::Ok(vm_addr),
            MemoryMapping::Aligned(m) => &m.common,
            MemoryMapping::Unaligned(m) => &m.common,
        };
        if let Some((index, region)) = self.find_region(vm_addr) {
            if let Some(host_addr) = region.vm_to_host(access_type, vm_addr, len) {
                return ProgramResult::Ok(host_addr);
            }
            let mut region = (*region).clone();
            let max_len = self
                .get_regions()
                .get(index.saturating_add(1))
                .map_or(u64::MAX, |next_region| next_region.vm_addr)
                .saturating_sub(region.vm_addr);
            (common.access_violation_handler)(&mut region, max_len, access_type, vm_addr, len);
            if let Some(host_addr) = region.vm_to_host(access_type, vm_addr, len) {
                if let Err(err) = self.replace_region(index, region) {
                    return ProgramResult::Err(err);
                }
                return ProgramResult::Ok(host_addr);
            }
        }
        let common = match &self {
            MemoryMapping::Identity => return ProgramResult::Ok(vm_addr),
            MemoryMapping::Aligned(m) => &m.common,
            MemoryMapping::Unaligned(m) => &m.common,
        };
        common.generate_access_violation(access_type, vm_addr, len)
    }

    /// Loads `size_of::<T>()` bytes from the given address.
    #[inline]
    pub fn load<T: Pod + Into<u64>>(&mut self, vm_addr: u64) -> ProgramResult {
        let len = mem::size_of::<T>() as u64;
        debug_assert!(len <= mem::size_of::<u64>() as u64);
        match self.map_with_access_violation_handler(AccessType::Load, vm_addr, len) {
            ProgramResult::Ok(host_addr) => {
                ProgramResult::Ok(unsafe { ptr::read_unaligned::<T>(host_addr as *const T) }.into())
            }
            err => err,
        }
    }

    /// Store `value` at the given address.
    #[inline]
    pub fn store<T: Pod>(&mut self, value: T, vm_addr: u64) -> ProgramResult {
        let len = mem::size_of::<T>() as u64;
        debug_assert!(len <= mem::size_of::<u64>() as u64);
        match self.map_with_access_violation_handler(AccessType::Store, vm_addr, len) {
            ProgramResult::Ok(host_addr) => {
                unsafe { ptr::write_unaligned(host_addr as *mut T, value) };
                ProgramResult::Ok(host_addr)
            }
            err => err,
        }
    }

    /// Returns the `MemoryRegion` which may contain the given address.
    pub fn find_region(&self, vm_addr: u64) -> Option<(usize, &MemoryRegion)> {
        match self {
            MemoryMapping::Identity => None,
            MemoryMapping::Aligned(m) => m.find_region(vm_addr),
            MemoryMapping::Unaligned(m) => m.find_region(vm_addr),
        }
    }

    /// Returns the `MemoryRegion`s in this mapping.
    pub fn get_regions(&self) -> &[MemoryRegion] {
        match self {
            MemoryMapping::Identity => &[],
            MemoryMapping::Aligned(m) => &m.common.regions,
            MemoryMapping::Unaligned(m) => &m.common.regions,
        }
    }

    /// Replaces the `MemoryRegion` at the given index
    pub fn replace_region(&mut self, index: usize, region: MemoryRegion) -> Result<(), EbpfError> {
        let regions = self.get_regions();
        let next_region_start = regions
            .get(index.saturating_add(1))
            .map_or(u64::MAX, |next_region| next_region.vm_addr);
        if index >= regions.len()
            || regions[index].vm_addr != region.vm_addr
            || region.vm_addr_range().end > next_region_start
        {
            return Err(EbpfError::InvalidMemoryRegion(index));
        }
        match self {
            MemoryMapping::Identity => Err(EbpfError::InvalidMemoryRegion(index)),
            MemoryMapping::Aligned(m) => m.replace_region(index, region),
            MemoryMapping::Unaligned(m) => m.replace_region(index, region),
        }
    }
}

/// Fast, small linear cache used to speed up unaligned memory mapping.
#[derive(Debug)]
struct MappingCache {
    // The cached entries.
    entries: [(Range<u64>, usize); MappingCache::SIZE as usize],
    // Index of the last accessed memory region.
    //
    // New entries are written backwards, so that find() can always scan
    // forward which is faster.
    head: isize,
}

impl MappingCache {
    const SIZE: isize = 4;

    fn new() -> MappingCache {
        MappingCache {
            entries: array::from_fn(|_| (0..0, 0)),
            head: 0,
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    #[inline]
    fn find(&self, vm_addr: u64) -> Option<usize> {
        for i in 0..Self::SIZE {
            let index = (self.head + i) % Self::SIZE;
            // Safety:
            // index is guaranteed to be between 0..Self::SIZE
            let (vm_range, region_index) = unsafe { self.entries.get_unchecked(index as usize) };
            if vm_range.contains(&vm_addr) {
                return Some(*region_index);
            }
        }

        None
    }

    #[allow(clippy::arithmetic_side_effects)]
    #[inline]
    fn insert(&mut self, vm_range: Range<u64>, region_index: usize) {
        self.head = (self.head - 1).rem_euclid(Self::SIZE);
        // Safety:
        // self.head is guaranteed to be between 0..Self::SIZE
        unsafe { *self.entries.get_unchecked_mut(self.head as usize) = (vm_range, region_index) };
    }

    #[inline]
    fn flush(&mut self) {
        self.entries = array::from_fn(|_| (0..0, 0));
        self.head = 0;
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};
    use test_utils::assert_error;

    use super::*;

    #[test]
    fn test_mapping_cache() {
        let mut cache = MappingCache::new();
        assert_eq!(cache.find(0), None);

        let mut ranges = vec![10u64..20, 20..30, 30..40, 40..50];
        for (region, range) in ranges.iter().cloned().enumerate() {
            cache.insert(range, region);
        }
        for (region, range) in ranges.iter().enumerate() {
            if region > 0 {
                assert_eq!(cache.find(range.start - 1), Some(region - 1));
            } else {
                assert_eq!(cache.find(range.start - 1), None);
            }
            assert_eq!(cache.find(range.start), Some(region));
            assert_eq!(cache.find(range.start + 1), Some(region));
            assert_eq!(cache.find(range.end - 1), Some(region));
            if region < 3 {
                assert_eq!(cache.find(range.end), Some(region + 1));
            } else {
                assert_eq!(cache.find(range.end), None);
            }
        }

        cache.insert(50..60, 4);
        ranges.push(50..60);
        for (region, range) in ranges.iter().enumerate() {
            if region == 0 {
                assert_eq!(cache.find(range.start), None);
                continue;
            }
            if region > 1 {
                assert_eq!(cache.find(range.start - 1), Some(region - 1));
            } else {
                assert_eq!(cache.find(range.start - 1), None);
            }
            assert_eq!(cache.find(range.start), Some(region));
            assert_eq!(cache.find(range.start + 1), Some(region));
            assert_eq!(cache.find(range.end - 1), Some(region));
            if region < 4 {
                assert_eq!(cache.find(range.end), Some(region + 1));
            } else {
                assert_eq!(cache.find(range.end), None);
            }
        }
    }

    #[test]
    fn test_mapping_cache_flush() {
        let mut cache = MappingCache::new();
        assert_eq!(cache.find(0), None);
        cache.insert(0..10, 0);
        assert_eq!(cache.find(0), Some(0));
        cache.flush();
        assert_eq!(cache.find(0), None);
    }

    #[test]
    fn test_map_empty() {
        for aligned_memory_mapping in [false, true] {
            let config = Config {
                aligned_memory_mapping,
                ..Config::default()
            };
            let m = MemoryMapping::new(vec![], &config, SBPFVersion::V3).unwrap();
            assert_error!(
                m.map(AccessType::Load, ebpf::MM_INPUT_START, 8),
                "AccessViolation"
            );
        }
    }

    #[test]
    fn test_gapped_map() {
        for aligned_memory_mapping in [false, true] {
            let config = Config {
                aligned_memory_mapping,
                ..Config::default()
            };
            let mut mem1 = vec![0xff; 8];
            let mut m = MemoryMapping::new(
                vec![
                    MemoryRegion::new_readonly(&[0; 8], ebpf::MM_RODATA_START),
                    MemoryRegion::new_writable_gapped(&mut mem1, ebpf::MM_STACK_START, 2),
                ],
                &config,
                SBPFVersion::V3,
            )
            .unwrap();
            for frame in 0..4 {
                let address = ebpf::MM_STACK_START + frame * 4;
                assert!(m.find_region(address).is_some());
                assert!(m.map(AccessType::Load, address, 2).is_ok());
                assert_error!(m.map(AccessType::Load, address + 2, 2), "AccessViolation");
                assert_eq!(m.load::<u16>(address).unwrap(), 0xFFFF);
                assert_error!(m.load::<u16>(address + 2), "AccessViolation");
                assert!(m.store::<u16>(0xFFFF, address).is_ok());
                assert_error!(m.store::<u16>(0xFFFF, address + 2), "AccessViolation");
            }
        }
    }

    #[test]
    fn test_unaligned_map_overlap() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };
        let mem1 = [1, 2, 3, 4];
        let mem2 = [5, 6];
        assert_error!(
            MemoryMapping::new(
                vec![
                    MemoryRegion::new_readonly(&mem1, ebpf::MM_INPUT_START),
                    MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + mem1.len() as u64 - 1),
                ],
                &config,
                SBPFVersion::V3,
            ),
            "InvalidMemoryRegion(1)"
        );
        assert!(MemoryMapping::new(
            vec![
                MemoryRegion::new_readonly(&mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + mem1.len() as u64),
            ],
            &config,
            SBPFVersion::V3,
        )
        .is_ok());
    }

    #[test]
    fn test_unaligned_map() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };
        let mut mem1 = [11];
        let mem2 = [22, 22];
        let mem3 = [33];
        let mem4 = [44, 44];
        let m = MemoryMapping::new(
            vec![
                MemoryRegion::new_writable(&mut mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + mem1.len() as u64),
                MemoryRegion::new_readonly(
                    &mem3,
                    ebpf::MM_INPUT_START + (mem1.len() + mem2.len()) as u64,
                ),
                MemoryRegion::new_readonly(
                    &mem4,
                    ebpf::MM_INPUT_START + (mem1.len() + mem2.len() + mem3.len()) as u64,
                ),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();

        assert_eq!(
            m.map(AccessType::Load, ebpf::MM_INPUT_START, 1).unwrap(),
            mem1.as_ptr() as u64
        );

        assert_eq!(
            m.map(AccessType::Store, ebpf::MM_INPUT_START, 1).unwrap(),
            mem1.as_ptr() as u64
        );

        assert_error!(
            m.map(AccessType::Load, ebpf::MM_INPUT_START, 2),
            "AccessViolation"
        );

        assert_eq!(
            m.map(
                AccessType::Load,
                ebpf::MM_INPUT_START + mem1.len() as u64,
                1,
            )
            .unwrap(),
            mem2.as_ptr() as u64
        );

        assert_eq!(
            m.map(
                AccessType::Load,
                ebpf::MM_INPUT_START + (mem1.len() + mem2.len()) as u64,
                1,
            )
            .unwrap(),
            mem3.as_ptr() as u64
        );

        assert_eq!(
            m.map(
                AccessType::Load,
                ebpf::MM_INPUT_START + (mem1.len() + mem2.len() + mem3.len()) as u64,
                1,
            )
            .unwrap(),
            mem4.as_ptr() as u64
        );

        assert_error!(
            m.map(
                AccessType::Load,
                ebpf::MM_INPUT_START + (mem1.len() + mem2.len() + mem3.len() + mem4.len()) as u64,
                1,
            ),
            "AccessViolation"
        );
    }

    #[test]
    fn test_unaligned_region() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };

        let mut mem1 = vec![0xFF; 4];
        let mem2 = vec![0xDD; 4];
        let m = MemoryMapping::new(
            vec![
                MemoryRegion::new_writable(&mut mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + 4),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();
        assert!(m.find_region(ebpf::MM_INPUT_START - 1).is_none());
        assert_eq!(
            m.find_region(ebpf::MM_INPUT_START).unwrap().1.host_addr,
            mem1.as_ptr() as u64
        );
        assert_eq!(
            m.find_region(ebpf::MM_INPUT_START + 3).unwrap().1.host_addr,
            mem1.as_ptr() as u64
        );
        assert_eq!(
            m.find_region(ebpf::MM_INPUT_START + 4).unwrap().1.host_addr,
            mem2.as_ptr() as u64
        );
        assert_eq!(
            m.find_region(ebpf::MM_INPUT_START + 7).unwrap().1.host_addr,
            mem2.as_ptr() as u64
        );
        assert!(m.find_region(ebpf::MM_INPUT_START + 8).is_some());
    }

    #[test]
    fn test_aligned_region() {
        let config = Config {
            aligned_memory_mapping: true,
            ..Config::default()
        };

        let mut mem1 = vec![0xFF; 4];
        let mem2 = vec![0xDD; 4];
        let m = MemoryMapping::new(
            vec![
                MemoryRegion::new_writable(&mut mem1, ebpf::MM_RODATA_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_STACK_START),
            ],
            &config,
            SBPFVersion::V4,
        )
        .unwrap();
        assert!(m.find_region(ebpf::MM_RODATA_START - 1).is_none());
        assert_eq!(
            m.find_region(ebpf::MM_RODATA_START).unwrap().1.host_addr,
            mem1.as_ptr() as u64
        );
        assert_eq!(
            m.find_region(ebpf::MM_RODATA_START + 3)
                .unwrap()
                .1
                .host_addr,
            mem1.as_ptr() as u64
        );
        assert!(m.find_region(ebpf::MM_RODATA_START + 4).is_some());
        assert_eq!(
            m.find_region(ebpf::MM_STACK_START).unwrap().1.host_addr,
            mem2.as_ptr() as u64
        );
        assert_eq!(
            m.find_region(ebpf::MM_STACK_START + 3).unwrap().1.host_addr,
            mem2.as_ptr() as u64
        );
        assert!(m.find_region(ebpf::MM_INPUT_START + 4).is_none());
    }

    #[test]
    fn test_unaligned_map_load() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };
        let mem1 = [0x11, 0x22];
        let mem2 = [0x33];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_readonly(&mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + mem1.len() as u64),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();

        assert_eq!(m.load::<u16>(ebpf::MM_INPUT_START).unwrap(), 0x2211);
        assert_error!(m.load::<u32>(ebpf::MM_INPUT_START), "AccessViolation");
        assert_error!(m.load::<u32>(ebpf::MM_INPUT_START + 4), "AccessViolation");
    }

    #[test]
    fn test_unaligned_map_store() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };
        let mut mem1 = vec![0xff, 0xff];
        let mut mem2 = vec![0xff];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_writable(&mut mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_writable(&mut mem2, ebpf::MM_INPUT_START + mem1.len() as u64),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();

        m.store(0x1122u16, ebpf::MM_INPUT_START).unwrap();
        assert_eq!(m.load::<u16>(ebpf::MM_INPUT_START).unwrap(), 0x1122);

        assert_error!(
            m.store(0x33445566u32, ebpf::MM_INPUT_START),
            "AccessViolation"
        );
    }

    #[test]
    fn test_unaligned_map_store_out_of_bounds() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };

        let mut mem1 = vec![0xFF];
        let mut m = MemoryMapping::new(
            vec![MemoryRegion::new_writable(&mut mem1, ebpf::MM_INPUT_START)],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();
        m.store(0x11u8, ebpf::MM_INPUT_START).unwrap();
        assert_error!(m.store(0x11u8, ebpf::MM_INPUT_START - 1), "AccessViolation");
        assert_error!(m.store(0x11u8, ebpf::MM_INPUT_START + 1), "AccessViolation");
        // this gets us line coverage for the case where we're completely
        // outside the address space (the case above is just on the edge)
        assert_error!(m.store(0x11u8, ebpf::MM_INPUT_START + 2), "AccessViolation");

        let mut mem1 = vec![0xFF; 4];
        let mut mem2 = vec![0xDD; 4];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_writable(&mut mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_writable(&mut mem2, ebpf::MM_INPUT_START + 4),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();
        assert_error!(
            m.store(0x1122334455667788u64, ebpf::MM_INPUT_START),
            "AccessViolation"
        );
    }

    #[test]
    fn test_unaligned_map_load_out_of_bounds() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };

        let mem1 = vec![0xff];
        let mut m = MemoryMapping::new(
            vec![MemoryRegion::new_readonly(&mem1, ebpf::MM_INPUT_START)],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();
        assert_eq!(m.load::<u8>(ebpf::MM_INPUT_START).unwrap(), 0xff);
        assert_error!(m.load::<u8>(ebpf::MM_INPUT_START - 1), "AccessViolation");
        assert_error!(m.load::<u8>(ebpf::MM_INPUT_START + 1), "AccessViolation");
        assert_error!(m.load::<u8>(ebpf::MM_INPUT_START + 2), "AccessViolation");

        let mem1 = vec![0xFF; 4];
        let mem2 = vec![0xDD; 4];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_readonly(&mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + 4),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();
        assert_error!(m.load::<u64>(ebpf::MM_INPUT_START), "AccessViolation");
    }

    #[test]
    #[should_panic(expected = "AccessViolation")]
    fn test_store_readonly() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };
        let mut mem1 = vec![0xff, 0xff];
        let mem2 = vec![0xff, 0xff];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_writable(&mut mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + mem1.len() as u64),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();
        m.store(0x11223344, ebpf::MM_INPUT_START).unwrap();
    }

    #[test]
    fn test_unaligned_map_replace_region() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };
        let mem1 = [11];
        let mem2 = [22, 22];
        let mem3 = [33];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_readonly(&mem1, ebpf::MM_INPUT_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_INPUT_START + mem1.len() as u64),
            ],
            &config,
            SBPFVersion::V3,
        )
        .unwrap();

        assert_eq!(
            m.map(AccessType::Load, ebpf::MM_INPUT_START, 1).unwrap(),
            mem1.as_ptr() as u64
        );

        assert_eq!(
            m.map(
                AccessType::Load,
                ebpf::MM_INPUT_START + mem1.len() as u64,
                1,
            )
            .unwrap(),
            mem2.as_ptr() as u64
        );

        assert_error!(
            m.replace_region(
                2,
                MemoryRegion::new_readonly(&mem3, ebpf::MM_INPUT_START + mem1.len() as u64)
            ),
            "InvalidMemoryRegion(2)"
        );

        let region_index = m
            .get_regions()
            .iter()
            .position(|mem| mem.vm_addr == ebpf::MM_INPUT_START + mem1.len() as u64)
            .unwrap();

        // old.vm_addr != new.vm_addr
        assert_error!(
            m.replace_region(
                region_index,
                MemoryRegion::new_readonly(&mem3, ebpf::MM_INPUT_START + mem1.len() as u64 + 1)
            ),
            "InvalidMemoryRegion({})",
            region_index
        );

        m.replace_region(
            region_index,
            MemoryRegion::new_readonly(&mem3, ebpf::MM_INPUT_START + mem1.len() as u64),
        )
        .unwrap();

        assert_eq!(
            m.map(
                AccessType::Load,
                ebpf::MM_INPUT_START + mem1.len() as u64,
                1,
            )
            .unwrap(),
            mem3.as_ptr() as u64
        );
    }

    #[test]
    fn test_aligned_map_replace_region() {
        let config = Config {
            aligned_memory_mapping: true,
            ..Config::default()
        };
        let mem1 = [11];
        let mem2 = [22, 22];
        let mem3 = [33, 33];
        let mut m = MemoryMapping::new(
            vec![
                MemoryRegion::new_readonly(&mem1, ebpf::MM_RODATA_START),
                MemoryRegion::new_readonly(&mem2, ebpf::MM_STACK_START),
            ],
            &config,
            SBPFVersion::V4,
        )
        .unwrap();

        assert_eq!(
            m.map(AccessType::Load, ebpf::MM_STACK_START, 1).unwrap(),
            mem2.as_ptr() as u64
        );

        // index > regions.len()
        assert_error!(
            m.replace_region(3, MemoryRegion::new_readonly(&mem3, ebpf::MM_STACK_START)),
            "InvalidMemoryRegion(3)"
        );

        // index != addr >> VIRTUAL_ADDRESS_BITS
        assert_error!(
            m.replace_region(2, MemoryRegion::new_readonly(&mem3, ebpf::MM_HEAP_START)),
            "InvalidMemoryRegion(2)"
        );

        // index + len != addr >> VIRTUAL_ADDRESS_BITS
        assert_error!(
            m.replace_region(
                2,
                MemoryRegion::new_readonly(&mem3, ebpf::MM_HEAP_START - 1)
            ),
            "InvalidMemoryRegion(2)"
        );

        m.replace_region(2, MemoryRegion::new_readonly(&mem3, ebpf::MM_STACK_START))
            .unwrap();

        assert_eq!(
            m.map(AccessType::Load, ebpf::MM_STACK_START, 1).unwrap(),
            mem3.as_ptr() as u64
        );
    }

    #[test]
    fn test_access_violation_handler_map() {
        for aligned_memory_mapping in [true, false] {
            let config = Config {
                aligned_memory_mapping,
                ..Config::default()
            };
            let original = [11, 22];
            let copied = Rc::new(RefCell::new(Vec::new()));
            let mut regions = vec![MemoryRegion::new_readonly(&original, ebpf::MM_RODATA_START)];
            regions[0].access_violation_handler_payload = Some(0);

            let c = Rc::clone(&copied);
            let mut m = MemoryMapping::new_with_access_violation_handler(
                regions,
                &config,
                SBPFVersion::V3,
                Box::new(move |region, _, _, _, _| {
                    c.borrow_mut().extend_from_slice(&original);
                    region.writable = true;
                    region.host_addr = c.borrow().as_slice().as_ptr() as u64;
                }),
            )
            .unwrap();

            assert_eq!(
                m.map_with_access_violation_handler(AccessType::Load, ebpf::MM_RODATA_START, 1)
                    .unwrap(),
                original.as_ptr() as u64
            );
            assert_eq!(
                m.map_with_access_violation_handler(AccessType::Store, ebpf::MM_RODATA_START, 1)
                    .unwrap(),
                copied.borrow().as_ptr() as u64
            );
        }
    }

    #[test]
    fn test_access_violation_handler_load_store() {
        for aligned_memory_mapping in [true, false] {
            let config = Config {
                aligned_memory_mapping,
                ..Config::default()
            };
            let original = [11, 22];
            let copied = Rc::new(RefCell::new(Vec::new()));
            let mut regions = vec![MemoryRegion::new_readonly(&original, ebpf::MM_RODATA_START)];
            regions[0].access_violation_handler_payload = Some(0);

            let c = Rc::clone(&copied);
            let mut m = MemoryMapping::new_with_access_violation_handler(
                regions,
                &config,
                SBPFVersion::V3,
                Box::new(move |region, _, _, _, _| {
                    c.borrow_mut().extend_from_slice(&original);
                    region.writable = true;
                    region.host_addr = c.borrow().as_slice().as_ptr() as u64;
                }),
            )
            .unwrap();

            assert_eq!(
                m.map(AccessType::Load, ebpf::MM_RODATA_START, 1).unwrap(),
                original.as_ptr() as u64
            );

            assert_eq!(m.load::<u8>(ebpf::MM_RODATA_START).unwrap(), 11);
            assert_eq!(m.load::<u8>(ebpf::MM_RODATA_START + 1).unwrap(), 22);
            assert!(copied.borrow().is_empty());

            m.store(33u8, ebpf::MM_RODATA_START).unwrap();
            assert_eq!(original[0], 11);
            assert_eq!(m.load::<u8>(ebpf::MM_RODATA_START).unwrap(), 33);
            assert_eq!(m.load::<u8>(ebpf::MM_RODATA_START + 1).unwrap(), 22);
        }
    }

    #[test]
    fn test_access_violation_handler_region_id() {
        for aligned_memory_mapping in [true, false] {
            let config = Config {
                aligned_memory_mapping,
                ..Config::default()
            };
            let original1 = [11, 22];
            let original2 = [33, 44];
            let copied = Rc::new(RefCell::new(Vec::new()));

            let mut regions = vec![
                MemoryRegion::new_readonly(&original1, ebpf::MM_RODATA_START),
                MemoryRegion::new_readonly(&original2, ebpf::MM_RODATA_START + 0x100000000),
            ];
            regions[0].access_violation_handler_payload = Some(42);

            let c = Rc::clone(&copied);
            let mut m = MemoryMapping::new_with_access_violation_handler(
                regions,
                &config,
                SBPFVersion::V3,
                Box::new(move |region, _, _, _, _| {
                    // check that the argument passed to MemoryRegion::new_readonly is then passed to the
                    // callback
                    assert_eq!(region.access_violation_handler_payload, Some(42));
                    c.borrow_mut().extend_from_slice(&original1);
                    region.writable = true;
                    region.host_addr = c.borrow().as_slice().as_ptr() as u64;
                }),
            )
            .unwrap();

            m.store(55u8, ebpf::MM_RODATA_START).unwrap();
            assert_eq!(original1[0], 11);
            assert_eq!(m.load::<u8>(ebpf::MM_RODATA_START).unwrap(), 55);
        }
    }

    #[test]
    #[should_panic(expected = "AccessViolation")]
    fn test_map_access_violation_handler_error() {
        let config = Config::default();
        let original = [11, 22];

        let m = MemoryMapping::new_with_access_violation_handler(
            vec![MemoryRegion::new_readonly(&original, ebpf::MM_RODATA_START)],
            &config,
            SBPFVersion::V4,
            Box::new(default_access_violation_handler),
        )
        .unwrap();

        m.map(AccessType::Store, ebpf::MM_RODATA_START, 1).unwrap();
    }

    #[test]
    #[should_panic(expected = "AccessViolation")]
    fn test_store_access_violation_handler_error() {
        let config = Config::default();
        let original = [11, 22];

        let mut m = MemoryMapping::new_with_access_violation_handler(
            vec![MemoryRegion::new_readonly(&original, ebpf::MM_RODATA_START)],
            &config,
            SBPFVersion::V4,
            Box::new(default_access_violation_handler),
        )
        .unwrap();

        m.store(33u8, ebpf::MM_RODATA_START).unwrap();
    }

    #[test]
    fn v4_aligned_mapping() {
        let config = Config {
            aligned_memory_mapping: false,
            ..Config::default()
        };

        let mapping = MemoryMapping::new_with_access_violation_handler(
            vec![MemoryRegion::new_readonly(&[11, 12], ebpf::MM_RODATA_START)],
            &config,
            SBPFVersion::V4,
            Box::new(default_access_violation_handler),
        )
        .unwrap();

        assert!(matches!(mapping, MemoryMapping::Aligned(_)));
    }
}
