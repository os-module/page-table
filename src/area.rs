use alloc::string::ToString;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::ops::Range;
use crate::{PageNumber, PPN, VPN};

#[derive(Debug, Clone)]
pub struct Area {
    /// 映射区域的虚拟页
    vpn_s: Range<VPN>,
    ///
    ppn_s: Option<Range<PPN>>,
    /// 映射区域的权限
    permission: AreaPermission,
}

bitflags! {
    pub struct AreaPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

impl AreaPermission {
    pub fn from_str(s:&str)->Self{
        let mut permission = AreaPermission::empty();
        for c in s.chars(){
            match c{
                'r'=>permission |= AreaPermission::R,
                'w'=>permission |= AreaPermission::W,
                'x'=>permission |= AreaPermission::X,
                'u'=>permission |= AreaPermission::U,
                _=>{}
            }
        }
        permission
    }
}

#[macro_export]
/// 使用字符串描述区域的权限
/// #example
/// ```
/// use page_table::{ap_from_str, AreaPermission};
/// let ap = ap_from_str!("rwx");
/// assert_eq!(ap,AreaPermission::R|AreaPermission::W|AreaPermission::X);
/// ```
macro_rules! ap_from_str {
    ($s:expr) => {
        AreaPermission::from_str($s)
    };
}



impl Area {
    pub fn new(vpn_s:Range<VPN>, ppn_s:Option<Range<PPN>>, permission: AreaPermission) ->Self{
        // 保证虚拟页和物理页的数量相同
        if ppn_s.is_some(){
            assert_eq!(vpn_s.end - vpn_s.start, ppn_s.as_ref().unwrap().end - ppn_s.as_ref().unwrap().start);
        }
        Self{
            vpn_s,
            ppn_s,
            permission
        }
    }
    pub fn permission(&self)-> AreaPermission {
        self.permission
    }
    pub fn iter(&self)->MapAreaIter{
        MapAreaIter{
            vpn_s:self.vpn_s.clone(),
            ppn_s: self.ppn_s.clone()
        }
    }
    pub fn vpn_range(&self) -> Range<VPN>{
        self.vpn_s.clone()
    }
    pub fn ppn_range(&self) -> Option<Range<PPN>>{
        self.ppn_s.clone()
    }
}

pub struct MapAreaIter{
    vpn_s:Range<VPN>,
    ppn_s:Option<Range<PPN>>,
}

impl Iterator for MapAreaIter {
type Item = (VPN,Option<VPN>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.vpn_s.is_empty(){
            None
        }else {
            let vpn = self.vpn_s.start;
            let ppn = self.ppn_s.as_mut().map(|ppn_s| ppn_s.start);
            self.vpn_s.start += PageNumber(1);
            self.ppn_s.as_mut().map(|ppn_s| ppn_s.start += PageNumber(1));
            Some((vpn,ppn))
        }
    }
}



#[cfg(test)]
mod tests{
    use crate::area::AreaPermission;
    use crate::{Area, PPN, VPN};

    #[test]
    fn test_map_area_permission(){
        let bits = 0b0001_1110u8;
        let p = AreaPermission::from_bits(bits).unwrap();
        assert!(p.contains(AreaPermission::R));
        assert!(p.contains(AreaPermission::W));
        assert!(p.contains(AreaPermission::X));
        assert!(p.contains(AreaPermission::U));
    }
    #[test]
    fn test_map_area_permission_from_str(){
        let s = "rwxu";
        let p = AreaPermission::from_str(s);
        assert!(p.contains(AreaPermission::R));
        assert!(p.contains(AreaPermission::W));
        assert!(p.contains(AreaPermission::X));
        assert!(p.contains(AreaPermission::U));
    }
    #[test]
    fn test_map_area(){
        let rang = VPN::new(1)..VPN::new(100);
        let area = Area::new(rang.clone(), None, AreaPermission::from_str("x"));
        let mut i  = 1;
        area.iter().for_each(|(vpn,ppn)|{
            assert_eq!(vpn,VPN::new(i));
            i +=1;
            assert!(ppn.is_none());
        });
        let ppn_s = PPN::new(1)..PPN::new(100);
        let area = Area::new(rang, Some(ppn_s), AreaPermission::W);
        let mut i  = 1;
        area.iter().for_each(|(vpn,ppn)|{
            assert_eq!(vpn,VPN::new(i));
            assert_eq!(ppn.unwrap(),PPN::new(i));
            i +=1;
        });

    }
}