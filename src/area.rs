use alloc::vec::Vec;
use bitflags::bitflags;
use core::ops::Range;
use crate::{PPN, VPN};

#[derive(Debug, Clone)]
pub struct MapArea{
    /// 映射区域的虚拟页
    vpn_s: Range<VPN>,
    /// 映射区域的物理页
    ppn_s: Vec<PPN>,
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

impl  MapArea{
    pub fn new(vpn_s:Range<VPN>,ppn_s:Vec<PPN>,permission:MapAreaPermission)->Self{
        // 保证虚拟页和物理页的数量相同
        assert_eq!(vpn_s.end - vpn_s.start, ppn_s.len());
        Self{
            vpn_s,
            ppn_s,
            permission
        }
    }
    pub fn permission(&self)->MapAreaPermission{
        self.permission
    }
    pub fn iter(&self)->MapAreaIter{
        MapAreaIter{
            vpn_s:self.vpn_s.clone(),
            ppn_s:self.ppn_s.clone(),
            index:0
        }
    }
}

pub struct MapAreaIter{
    vpn_s:Range<VPN>,
    ppn_s:Vec<PPN>,
    index:usize
}
impl <T:MetaData> Iterator for MapAreaIter{
    type Item = (VPN,PPN);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.ppn_s.len(){
            let vpn = self.vpn_s.start + self.index.into();
            let ppn = self.ppn_s[self.index];
            self.index += 1;
            Some((vpn,ppn))
        }else{
            None
        }
    }
}