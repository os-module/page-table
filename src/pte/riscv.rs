//! RISC-V page table entries.

use crate::addr::PhysAddr;
use core::fmt;

use super::{GenericPTE, MappingFlags};

bitflags::bitflags! {
    /// Page-table entry flags.
    pub struct PTEFlags: usize {
        /// Whether the PTE is valid.
        const V =   1 << 0;
        /// Whether the page is readable.
        const R =   1 << 1;
        /// Whether the page is writable.
        const W =   1 << 2;
        /// Whether the page is executable.
        const X =   1 << 3;
        /// Whether the page is accessible to user mode.
        const U =   1 << 4;
        /// Designates a global mapping.
        const G =   1 << 5;
        /// Indicates the virtual page has been read, written, or fetched from
        /// since the last time the A bit was cleared.
        const A =   1 << 6;
        /// Indicates the virtual page has been written since the last time the
        /// D bit was cleared.
        const D =   1 << 7;
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(f: PTEFlags) -> Self {
        MappingFlags::from_bits_truncate(f.bits())
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(f: MappingFlags) -> Self {
        PTEFlags::from_bits_truncate(f.bits())
    }
}

/// Sv39 and Sv48 page table entry for RV64 systems.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Rv64PTE(u64);

impl Rv64PTE {
    const PHYS_ADDR_MASK: u64 = (1 << 54) - (1 << 10); // bits 10..54
}

impl GenericPTE for Rv64PTE {
    fn new_page(paddr: PhysAddr, flags: MappingFlags, _is_huge: bool) -> Self {
        let flags = PTEFlags::from(flags);
        debug_assert!(flags.intersects(PTEFlags::R | PTEFlags::X));
        Self(flags.bits() as u64 | ((paddr.as_usize() >> 2) as u64 & Self::PHYS_ADDR_MASK))
    }
    fn new_table(paddr: PhysAddr) -> Self {
        Self(PTEFlags::V.bits() as u64 | ((paddr.as_usize() >> 2) as u64 & Self::PHYS_ADDR_MASK))
    }
    fn paddr(&self) -> PhysAddr {
        PhysAddr::from(((self.0 & Self::PHYS_ADDR_MASK) << 2) as usize)
    }
    fn flags(&self) -> MappingFlags {
        PTEFlags::from_bits_truncate(self.0 as usize).into()
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        PTEFlags::from_bits_truncate(self.0 as usize).contains(PTEFlags::V)
    }
    fn is_huge(&self) -> bool {
        PTEFlags::from_bits_truncate(self.0 as usize).intersects(PTEFlags::R | PTEFlags::X)
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl fmt::Debug for Rv64PTE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("Rv64PTE");
        f.field("raw", &self.0)
            .field("paddr", &self.paddr())
            .field("flags", &self.flags())
            .finish()
    }
}