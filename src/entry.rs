use crate::{MetaData, PPN};
use bitflags::bitflags;
use core::marker::PhantomData;

pub struct PageTableEntry<T: MetaData> {
    entry: usize,
    meta: PhantomData<T>,
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
impl<T: MetaData> PageTableEntry<T> {
    pub fn new(ppn: PPN<T>, attr: PTEFlags) -> Self {
        let entry = ppn.0 << T::ppn_index_range().start | attr.bits() as usize;
        Self {
            entry,
            meta: PhantomData,
        }
    }
    fn flag(&self) -> PTEFlags {
        PTEFlags::from_bits(self.entry as u8).unwrap()
    }
}

impl<T: MetaData> PTELike for PageTableEntry<T> {
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
        (self.entry >> T::ppn_index_range().start) * T::PAGE_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PagingMode;
    #[derive(Copy, Clone)]
    struct Meta;

    impl MetaData for Meta {
        const PAGE_SIZE: usize = 0x1000;
        const PAGING_MODE: PagingMode = PagingMode::Sv39;
    }

    #[test]
    fn test_entry() {
        let entry = PageTableEntry::<Meta>::new(PPN::<Meta>::new(0), PTEFlags::V);
        assert!(entry.is_valid());
        assert!(!entry.is_read());
        assert!(!entry.is_write());
        assert!(!entry.is_exec());
        assert!(!entry.is_user());
        assert!(!entry.is_accessed());
        assert!(!entry.is_dirty());
        assert_eq!(entry.physical_address(), 0);
        let entry = PageTableEntry::<Meta>::new(PPN::<Meta>::new(1), PTEFlags::V | PTEFlags::R);
        assert!(entry.is_valid());
        assert!(entry.is_read());
        assert_eq!(entry.physical_address(), 0x1000);
    }
}
