use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use core::ops::{Deref, Range};
use log::{error, info};
use crate::table::PageTable;
use crate::{PageManager, PPN, VPN, vpn_f_c_range, VPNToSlice};
use crate::area::Area;
use crate::entry::{PageTableEntry, PTEFlags, PTEFlagsBuilder, PTELike};
use crate::error::PTableError;
use crate::Result;


#[derive(Clone)]
pub struct AddressSpace {
    /// the root ppn
    root_ppn: Option<PPN>,
    map_area:Vec<Area>,
    single_vpn:Vec<(VPN,PTEFlags)>,
    page_manager:InternalManager
}

#[derive(Clone)]
struct InternalManager{
    manager:Arc<dyn PageManager>,
    // record the ppn which is used by this address space
    ppn_set:Vec<PPN>,
}
impl InternalManager{
    pub fn new(manager:Arc<dyn PageManager>)->Self{
        Self{
            manager,
            ppn_set:Vec::new(),
        }
    }
}
impl InternalManager{
    fn alloc(&mut self) -> Option<PPN> {
        let res = self.manager.alloc();
        if res.is_some(){
            self.ppn_set.push(res.clone().unwrap());
        }
        res
    }
    fn dealloc(&self, ppn: PPN) {
        self.manager.dealloc(ppn)
    }
}

impl AddressSpace{
    pub fn new(page_manager:Arc<dyn PageManager>)->Self{
        Self{
            root_ppn:None,
            map_area:Vec::new(),
            single_vpn: vec![],
            page_manager:InternalManager::new(page_manager)
        }
    }

    pub fn root_ppn(&self)->Option<PPN>{
        self.root_ppn.clone()
    }

    pub fn recycle(&mut self){
        self.page_manager.ppn_set.iter().for_each(|x|{
            self.page_manager.dealloc(*x)
        })
    }

    pub fn copy_from_other(address_space:&AddressSpace)->Result<Self>{
        let mut new_address_space = Self::new(address_space.page_manager.manager.clone());
        for area in address_space.map_area.iter(){
            let permission = area.permission();
            let vpn_range = area.vpn_range();
            let new_area = Area::new(vpn_range.clone(),None,permission);
            new_address_space.push(new_area);
            for vpn in vpn_range{
                let ppn = new_address_space.vpn_to_ppn(vpn).unwrap();
                let new_physical = ppn.to_address() as * mut u8;
                let old_physical = address_space.vpn_to_ppn(vpn).unwrap().to_address() as * const u8;
                unsafe {
                    new_physical.copy_from(old_physical,4096);
                }
            }
        }
        for (vpn,perm) in address_space.single_vpn.iter(){
            let new_ppn = new_address_space.push_with_vpn(*vpn,*perm)?;
            let old_ppn = address_space.vpn_to_ppn(*vpn).unwrap();
            let new_physical = new_ppn.to_address() as * mut u8;
            let old_physical = old_ppn.to_address() as * const u8;
            unsafe {
                new_physical.copy_from(old_physical,4096)
            }
        };
        Ok(new_address_space)
    }

    fn check_root(&mut self)->Result<()>{
        if self.root_ppn.is_none(){
            let ppn = self.page_manager.alloc().ok_or(PTableError::AllocError)?;
            self.root_ppn = Some(ppn);
        }
        Ok(())
    }

    pub fn push(&mut self, map_area: Area){
        self.map(&map_area,false,true);
        self.map_area.push(map_area);
    }

    fn map(&mut self, map_area:&Area, flag:bool,is_valid:bool) ->Vec<PPN>{
        self.check_root().unwrap();
        let map_permission = map_area.permission().bits();
        let mut map = Vec::new();
        for (vpn,ppn) in map_area.iter(){
            let slice = vpn.to_slice();
            // 查找页表项并映射
            let mut page_table = PageTable::from_ppn(self.root_ppn.unwrap());
            for i in 0..slice.len()-1{
                let pte = page_table[slice[i]];
                if !pte.is_valid(){
                    let new_ppn = self.page_manager.alloc().unwrap();
                    // 页表项无效，分配新的页表
                    // 非叶子节点需要保证RWX位为0
                    page_table[slice[i]] = PageTableEntry::new(new_ppn,PTEFlags::V);
                    page_table = PageTable::from_ppn(new_ppn);
                }else{
                    page_table = PageTable::from_ppn(pte.ppn());
                }
            }
            //填充叶子节点
            let ppn = if ppn.is_some(){
                ppn.unwrap()
            }else {
                self.page_manager.alloc().unwrap()
            };
            let valid = if is_valid{
                PTEFlags::V
            }else {
                PTEFlags::empty()
            };
            page_table[slice[slice.len()-1]] = PageTableEntry::new(ppn,valid|PTEFlags::from_bits(map_permission).unwrap());
            if flag{
                map.push(ppn);
            }
        };
        map
    }
    pub fn tmp_push(&mut self, map_area: Area,is_valid:bool){
        self.map(&map_area,false,is_valid);
        self.map_area.push(map_area);
    }

    /// The vpn is unvaild, we need to make it valid
    pub fn tmp_make_valid(&self,vpn:VPN){
        let slice = vpn.to_slice();
        let mut page_table = PageTable::from_ppn(self.root_ppn.unwrap());
        for i in 0..slice.len()-1{
            let pte = page_table[slice[i]];
            assert!(pte.is_valid());
            page_table = PageTable::from_ppn(pte.ppn());
        }
        let pte = page_table[slice[slice.len()-1]];
        assert!(!pte.is_valid());
        let flag = pte.flag();
        page_table[slice[slice.len()-1]] = PageTableEntry::new(pte.ppn(),flag|PTEFlags::V);
    }



    /// 添加逻辑段并拷贝数据
    pub fn push_with_data(&mut self, map_area: Area, data:&[u8]){
        let v_to_p = self.map(&map_area,true,true);
        self.map_area.push(map_area);
        // 拷贝数据
        // vpn保证了键值对的顺序
        let mut start = 0;
        let len = data.len();
        for ppn in v_to_p{
            let addr = ppn.to_address() as *mut u8;
            let len = core::cmp::min(len-start,4096);
            unsafe{
                core::ptr::copy(data.as_ptr().add(start),addr,len);
            }
            start += len;
        }
    }

    pub fn push_with_vpn(&mut self, vpn:VPN, permission:PTEFlags)->Result<PPN>{
        self.check_root()?;
        let slice = vpn.to_slice();
        let mut page_table = PageTable::from_ppn(self.root_ppn.unwrap());
        for i in 0..slice.len()-1{
            let pte = page_table[slice[i]];
            if !pte.is_valid(){
                let new_ppn = self.page_manager.alloc().unwrap();
                // if the pte is invalid, alloc a new frame
                // the non-leaf node should ensure the RWX bit is 0
                page_table[slice[i]] = PageTableEntry::new(new_ppn,PTEFlags::V);
                page_table = PageTable::from_ppn(new_ppn);
            }else{
                page_table = PageTable::from_ppn(pte.ppn());
            }
        }
        // fill the leaf node
        let ppn = self.page_manager.alloc().unwrap();
        page_table[slice[slice.len()-1]] = PageTableEntry::new(ppn,permission);
        self.single_vpn.push((vpn,permission));
        Ok(ppn)
    }

    pub fn vpn_to_ppn(&self,vpn:VPN)->Option<PPN>{
        let slice = vpn.to_slice();
        let mut page_table = PageTable::from_ppn(self.root_ppn.unwrap());
        for i in 0..slice.len()-1{
            let pte = page_table[slice[i]];
            if !pte.is_valid(){
                return None;
            }else{
                page_table = PageTable::from_ppn(pte.ppn());
            }
        }
        let pte = page_table[slice[slice.len()-1]];
        if !pte.is_valid(){
            None
        }else{
            Some(pte.ppn())
        }
    }
    pub fn virtual_to_physical(&self,virtual_address:usize)->Option<usize> {
        let vpn = VPN::floor_address(virtual_address);
        self.vpn_to_ppn(vpn).map(|ppn| ppn.to_address() + (virtual_address & 0xfff))
    }

    /// unmap area
    ///
    /// User should ensure that the area is mapped. User can find the area by vpn.
    pub fn unmap(&mut self, map_area: &Area)->Result<()>{
        let mut page_table = PageTable::from_ppn(self.root_ppn.unwrap());
        for (vpn,_) in map_area.iter(){
            let slice = vpn.to_slice();
            for i in 0..slice.len()-1{
                let pte = page_table[slice[i]];
                if !pte.is_valid(){
                    return Err(PTableError::NotValid);
                }else{
                    page_table = PageTable::from_ppn(pte.ppn());
                }
            }
            page_table[slice[slice.len()-1]] = PageTableEntry::new(PPN::new(0),PTEFlags::from_bits(0).unwrap());
        }
        // delete map_area
        self.map_area.retain(|area| area != map_area);
        Ok(())
    }

    /// find area by vpn
    pub fn find_area(&self,vpn:VPN)->Option<&Area>{
        for area in self.map_area.iter(){
            if area.vpn_range().contains(&vpn){
                return Some(area);
            }
        }
        None
    }

    pub fn unmap_with_vpn(&mut self,vpn:VPN)->Result<()>{
        let mut  page_table = PageTable::from_ppn(self.root_ppn.unwrap());
        let slice = vpn.to_slice();
        for i in 0..slice.len()-1{
            let pte = page_table[slice[i]];
            if !pte.is_valid(){
                return Err(PTableError::NotValid);
            }else{
                page_table = PageTable::from_ppn(pte.ppn());
            }
        }
        page_table[slice[slice.len()-1]] = PageTableEntry::new(PPN::new(0),PTEFlags::from_bits(0).unwrap());
        Ok(())
    }
}

/// 打印多级页表并展示权限
impl Debug for AddressSpace{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f,"AddressSpace:\n")?;
        if let Some(root_ppn) = self.root_ppn{
            let mut page_table = PageTable::from_ppn(root_ppn);
            for i in 0..512{
                let pte = page_table[i];
                if pte.is_valid(){
                    write!(f,"{:03x}:{:012x}{:<5}",i,pte.ppn().0," ==> ")?;
                    let mut page_table = PageTable::from_ppn(pte.ppn());
                    let mut flag = -1;
                    for j in 0..512{
                        let pte = page_table[j];
                        if pte.is_valid(){
                            // 第一个有效页表项打印对齐大小为0
                            flag +=1;
                            let space = if flag==0 {0}else { 21 };
                            write!(f,"{1:0$}{2:03x}:{3:012x} ==> ",space,"",j,pte.ppn().0)?;
                            let mut page_table = PageTable::from_ppn(pte.ppn());
                            let mut flag1 = -1;
                            for k in 0..512{
                                let pte = page_table[k];
                                if pte.is_valid(){
                                    flag1 +=1;
                                    let space = if flag1==0{0}else { 42 };
                                    write!(f,"{1:0$}{2:03x}:{3:012x} {4:<8?}\n",space,"",k,pte.ppn().0,PTEFlagsBuilder(pte.flag()))?;
                                }
                            }
                            write!(f,"{:->67}\n","-")?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests{
    use alloc::collections::BTreeMap;
    use core::ops::Range;
    use crate::{PPN, VPN};

    #[test]
    fn test_(){
        let vpn_s = VPN::new(1) .. VPN::new(100);
        let mut map = BTreeMap::new();
        for vpn in vpn_s{
            map.insert(vpn,vpn.0+1);
        }
        let mut i = 1;
        map.iter().for_each(|(key,value)|{
            assert_eq!(*key,i.into());
            i = i+1;
        })
    }

}