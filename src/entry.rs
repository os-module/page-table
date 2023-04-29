use core::fmt::{Debug, Formatter, write};
use crate::{PPN};
use bitflags::bitflags;
use core::marker::PhantomData;

#[derive(Copy, Clone,Debug)]
#[repr(C)]
pub struct PageTableEntry{
    entry: usize,
}

bitflags! {
    /// 页表项标志位定义
    ///
    /// 在rv64和rv32中这些标志位都存在
    pub struct PTEFlags:u8{
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

pub struct PTEFlagsBuilder(pub PTEFlags);

impl Debug for PTEFlagsBuilder{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}{}{}{}{}{}{}{}",
               if self.0.contains(PTEFlags::D) { "D" } else { "-" },
               if self.0.contains(PTEFlags::A) { "A" } else { "-" },
               if self.0.contains(PTEFlags::G) { "G" } else { "-" },
               if self.0.contains(PTEFlags::U) { "U" } else { "-" },
               if self.0.contains(PTEFlags::X) { "X" } else { "-" },
               if self.0.contains(PTEFlags::W) { "W" } else { "-" },
               if self.0.contains(PTEFlags::R) { "R" } else { "-" },
               if self.0.contains(PTEFlags::V) { "V" } else { "-" },
        )
    }
}


pub trait PTELike {
    fn is_valid(&self) -> bool;
    fn is_read(&self) -> bool;
    fn is_write(&self) -> bool;
    fn is_exec(&self) -> bool;
    fn is_user(&self) -> bool;
    fn is_accessed(&self) -> bool;
    fn is_dirty(&self) -> bool;
    fn physical_address(&self) -> usize;
}
impl PageTableEntry {
    pub fn new(ppn: PPN, attr: PTEFlags) -> Self {
        let entry = ppn.0 << 10 | attr.bits() as usize;
        Self {
            entry,
        }
    }
    pub fn flag(&self) -> PTEFlags {
        PTEFlags::from_bits(self.entry as u8).unwrap()
    }
    pub fn empty() -> Self {
        Self {
            entry: 0,
        }
    }
    pub fn ppn(&self)->PPN{
        (self.entry >> 10).into()
    }
}

impl PTELike for PageTableEntry {
    fn is_valid(&self) -> bool {
        self.flag().contains(PTEFlags::V)
    }
    fn is_read(&self) -> bool {
        self.flag().contains(PTEFlags::R)
    }

    fn is_write(&self) -> bool {
        self.flag().contains(PTEFlags::W)
    }

    fn is_exec(&self) -> bool {
        self.flag().contains(PTEFlags::X)
    }

    fn is_user(&self) -> bool {
        self.flag().contains(PTEFlags::U)
    }

    fn is_accessed(&self) -> bool {
        self.flag().contains(PTEFlags::A)
    }

    fn is_dirty(&self) -> bool {
        self.flag().contains(PTEFlags::D)
    }

    fn physical_address(&self) -> usize {
        (self.entry >> 10) * 0x1000
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_entry() {
        let entry = PageTableEntry::new(PPN::new(0), PTEFlags::V);
        assert!(entry.is_valid());
        assert!(!entry.is_read());
        assert!(!entry.is_write());
        assert!(!entry.is_exec());
        assert!(!entry.is_user());
        assert!(!entry.is_accessed());
        assert!(!entry.is_dirty());
        assert_eq!(entry.physical_address(), 0);
        let entry = PageTableEntry::new(PPN::new(1), PTEFlags::V | PTEFlags::R);
        assert!(entry.is_valid());
        assert!(entry.is_read());
        assert_eq!(entry.physical_address(), 0x1000);
        assert_eq!(core::mem::size_of_val(&entry), core::mem::size_of::<usize>());
    }
}
