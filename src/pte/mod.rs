mod riscv;

use crate::addr::PhysAddr;
use core::fmt::Debug;

pub use self::riscv::*;

bitflags::bitflags! {
    /// Generic page table entry flags that indicate the corresponding mapped
    /// memory region permissions and attributes.
    pub struct MappingFlags: usize {
        const V         = 1 << 0;
        /// The memory is readable.
        const R          = 1 << 1;
        /// The memory is writable.
        const W         = 1 << 2;
        /// The memory is executable.
        const X      = 1 << 3;
        /// The memory is user accessible.
        const U          = 1 << 4;
        const G        = 1 << 5;
        const A      = 1 << 6;
        const D         = 1 << 7;
        const RSD = 1 << 8 | 1 << 9;
    }
}


impl From<&str> for MappingFlags{
    fn from(value: &str) -> Self {
        let mut ret = Self::empty();
        for c in value.chars() {
            match c {
                'V' => ret |= Self::V,
                'R' => ret |= Self::R,
                'W' => ret |= Self::W,
                'X' => ret |= Self::X,
                'U' => ret |= Self::U,
                'G' => ret |= Self::G,
                'A' => ret |= Self::A,
                'D' => ret |= Self::D,
                _ => panic!("Invalid MappingFlags"),
            }
        }
        ret
    }
}





/// A generic page table entry.
///
/// All architecture-specific page table entry types implement this trait.
pub trait GenericPTE: Debug + Clone + Copy + Sync + Send + Sized {
    /// Creates a page table entry point to a terminate page or block.
    fn new_page(paddr: PhysAddr, flags: MappingFlags, is_huge: bool) -> Self;
    /// Creates a page table entry point to a next level page table.
    fn new_table(paddr: PhysAddr) -> Self;

    /// Returns the physical address mapped by this entry.
    fn paddr(&self) -> PhysAddr;
    /// Returns the flags of this entry.
    fn flags(&self) -> MappingFlags;
    /// Returns whether this entry is zero.
    fn is_unused(&self) -> bool;
    /// Returns whether this entry flag indicates present.
    fn is_present(&self) -> bool;
    /// For non-last level translation, returns whether this entry maps to a
    /// huge frame.
    fn is_huge(&self) -> bool;
    /// Set this entry to zero.
    fn clear(&mut self);
}
