use crate::{MetaData, PPN};
use core::marker::PhantomData;

pub struct PageTable<T: MetaData> {
    /// 根页表的物理页号
    root: PPN<T>,
    /// 页表的元数据
    meta: PhantomData<T>,
}

impl<T: MetaData> PageTable<T> {
    /// 新建一个页表
    pub fn new(root_ppn: PPN<T>) -> Self {
        Self {
            root: root_ppn,
            meta: PhantomData,
        }
    }

    /// 获取根页表的物理页号
    fn root_ppn(&self) -> PPN<T> {
        self.root
    }
}
