use crate::table::PageTable;
use crate::MetaData;

pub struct AddressSpace<T: MetaData> {
    page_table: PageTable<T>,
}
