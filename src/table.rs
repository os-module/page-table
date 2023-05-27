use crate::entry::PageTableEntry;
use crate::PPN;
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

#[derive(Debug)]
#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); 512],
        }
    }
    pub fn from_ppn(ppn: PPN) -> &'static mut Self {
        unsafe { &mut *(ppn.to_address() as *mut Self) }
    }
    pub fn iter(&self) -> core::slice::Iter<'_, PageTableEntry> {
        self.entries.iter()
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < 512);
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < 512);
        &mut self.entries[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::PTELike;

    #[test]
    fn test_page_table() {
        let mut page_table = PageTable::new();
        page_table
            .iter()
            .for_each(|entry| assert!(!entry.is_valid()));
    }
}
