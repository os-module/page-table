use crate::{MetaData, PPN};
use core::marker::PhantomData;
use crate::entry::PageTableEntry;

pub struct PageTable {
    entries: [PageTableEntry;512]
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); N],
        }
    }
    pub fn from_ppn(ppn: PPN) -> &'static mut Self {
        unsafe { &mut *(ppn.to_address() as *mut Self) }
    }
    pub fn iter(&self) -> core::slice::Iter<'_, PageTableEntry> {
        self.entries.iter()
    }
}




#[cfg(test)]
mod tests{
    use crate::entry::PTELike;
    use crate::PagingMode;
    use super::*;

    #[test]
    fn test_page_table(){
        let mut page_table = PageTable::new();
        page_table.iter().for_each(|entry|{
            assert!(!entry.is_valid())
        });
    }
}
