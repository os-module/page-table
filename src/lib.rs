#![no_std]
#![allow(unused)]
use core::marker::PhantomData;
use core::ops::Range;

mod area;
mod entry;
mod space;
mod table;

pub enum PagingMode {
    #[cfg(feature = "rv32")]
    Sv32,
    #[cfg(feature = "rv64")]
    Sv39,
    #[cfg(feature = "rv64")]
    Sv48,
}

pub trait MetaData: Copy + Clone {
    const PAGE_SIZE: usize;
    const PAGING_MODE: PagingMode;
    fn entry_size() -> usize {
        match Self::PAGING_MODE {
            #[cfg(feature = "rv32")]
            PagingMode::Sv32 => 4,
            #[cfg(feature = "rv64")]
            PagingMode::Sv39 => 8,
            #[cfg(feature = "rv64")]
            PagingMode::Sv48 => 8,
        }
    }
    fn entry_per_page() -> usize {
        Self::PAGE_SIZE / Self::entry_size()
    }
    fn page_size() -> usize {
        Self::PAGE_SIZE
    }
    fn ppn_index_range() -> Range<usize> {
        match Self::PAGING_MODE {
            #[cfg(feature = "rv32")]
            PagingMode::Sv32 => 10..32,
            #[cfg(feature = "rv64")]
            PagingMode::Sv39 => 10..39,
            #[cfg(feature = "rv64")]
            PagingMode::Sv48 => 10..48,
        }
    }
}

type PhyAddr = usize;
type VirtAddr = usize;

#[derive(Copy, Clone, Debug)]
pub struct PageNumber<T: MetaData>(usize, PhantomData<T>);

pub type PPN<T> = PageNumber<T>;
pub type VPN<T> = PageNumber<T>;

impl<T: MetaData> PageNumber<T> {
    pub fn new(num: usize) -> Self {
        let ppn_index_range = T::ppn_index_range();
        let ppn_range = 0..1 << (ppn_index_range.end - ppn_index_range.start);
        assert!(ppn_range.contains(&num));
        Self(num, PhantomData)
    }
    pub fn to_address(&self) -> usize {
        self.0 * T::page_size()
    }
}
