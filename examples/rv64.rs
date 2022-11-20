use page_table::{MetaData, PagingMode, VPN};
use std::ops::Range;

fn main() {
    let vpn = VPN::<Meta>::new(123);
    // 新建一个地址空间
    // let address_space = AddressSpace::new();
    // // 新建一个map_area
    // let kernel = 0..2usize;
    // let map_area = MapArea::new(kernel,MapAreaPermission::RWX);
    // // 将map_area映射到地址空间
    // address_space.map(map_area);
    // // 删除某个映射
    // address_space.unmap(kernel);
}

#[derive(Copy, Clone, Debug)]
struct Meta;
impl MetaData for Meta {
    const PAGE_SIZE: usize = 4096;
    const PAGING_MODE: PagingMode = PagingMode::Sv39;
}
