use bitflags::bitflags;
use core::ops::Range;

pub struct MapArea {
    /// 映射区域的虚拟地址
    virtual_address: Range<usize>,
    /// 映射区域的权限
    permission: MapAreaPermission,
}

bitflags! {
    pub struct MapAreaPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}
