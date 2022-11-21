use alloc::vec::Vec;
use core::ops::Range;
use crate::table::PageTable;
use crate::{ PPN, VPNToSlice};
use crate::area::MapArea;

#[derive(Clone)]
pub struct AddressSpace {
    /// 根页表地址
    root_ppn: PPN,
    map_area:Vec<MapArea>
}


impl  AddressSpace{
    pub fn new(root_ppn:PPN)->Self{
        Self{
            root_ppn,
            map_area:Vec::new()
        }
    }
    pub fn push(&mut self,map_area:MapArea){
        self.map(&map_area);
        self.map_area.push(map_area);
    }
    fn map(&self,map_area:&MapArea){
        map_area.iter().for_each(|(vpn,ppn)|{
            let slice = vpn.to_slice();
        })
    }

}
