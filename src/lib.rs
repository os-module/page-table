#![feature(step_trait)]
#![feature(error_in_core)]
#![cfg_attr(not(test), no_std)]
#![allow(unused)]
extern crate alloc;
use alloc::vec::Vec;
use core::iter::Step;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Range, Sub};

mod area;
mod entry;
mod error;
mod space;
mod table;

pub type PPN = PageNumber;
pub type VPN = PageNumber;

pub use area::{Area, AreaPermission};
pub use entry::PTEFlags;
pub use error::PTableError;
pub use space::AddressSpace;

type Result<T> = core::result::Result<T, PTableError>;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct PageNumber(pub usize);

pub trait VPNToSlice {
    fn to_slice(&self) -> [usize; 3];
}

impl PageNumber {
    pub fn new(num: usize) -> Self {
        Self(num)
    }
    pub fn to_address(&self) -> usize {
        self.0 << 12
    }
    pub fn floor_address(address: usize) -> Self {
        Self(address >> 12)
    }
    pub fn ceil_address(address: usize) -> Self {
        Self((address + 4095) >> 12)
    }
}

impl Sub for PageNumber {
    type Output = usize;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add for PageNumber {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}

impl From<usize> for PageNumber {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl VPNToSlice for PageNumber {
    fn to_slice(&self) -> [usize; 3] {
        let mut slice = [0; 3];
        slice[0] = (self.0 >> 18) & 0x1ff;
        slice[1] = (self.0 >> 9) & 0x1ff;
        slice[2] = self.0 & 0x1ff;
        slice
    }
}

impl Step for PageNumber {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(end.0 - start.0)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::new(start.0 + count))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::new(start.0 - count))
    }
}

impl AddAssign for PageNumber {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

pub trait PageManager: Send + Sync {
    fn alloc(&self) -> Option<PPN>;
    fn dealloc(&self, ppn: PPN);
}

#[macro_export]
/// 将一个地址区间转换为虚拟页号区间
/// 起始地址向下取整，结束地址向上取整
/// # Example
/// ```
/// use page_table::{vpn_f_c_range,VPN};
/// let range = vpn_f_c_range!(0x1000, 0x2000);
/// assert_eq!(range, VPN::new(1)..VPN::new(2));
/// ```
macro_rules! vpn_f_c_range {
    ($start:expr, $end:expr) => {
        VPN::floor_address($start)..VPN::ceil_address($end)
    };
}

#[macro_export]
/// 将一个地址区间转换为虚拟页号区间
/// 起始地址向下取整，结束地址向上取整
/// # Example
/// ```
/// use page_table::{ppn_f_c_range,PPN};
/// let range = ppn_f_c_range!(0x1000, 0x2000);
/// assert_eq!(range, PPN::new(1)..PPN::new(2));
/// ```
macro_rules! ppn_f_c_range {
    ($start:expr, $end:expr) => {
        PPN::floor_address($start)..PPN::ceil_address($end)
    };
}

#[cfg(test)]
mod tests {
    use crate::{PageNumber, VPNToSlice, PPN, VPN};

    #[test]
    fn test_vpn_to_slice() {
        let vpn = VPN::new(0b111_111_111_111_111_111_111_111_111);
        let slice = vpn.to_slice();
        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0], 511);
        assert_eq!(slice[1], 511);
        assert_eq!(slice[2], 511);
        let num = 0b000_000_001_111_111_111_000_000_001usize;
        let vpn: VPN = num.into();
        let slice = vpn.to_slice();
        assert_eq!(slice[0], 1);
        assert_eq!(slice[1], 511);
        assert_eq!(slice[2], 1);
        let vpn = VPN::new(0x80200);
        let slice = vpn.to_slice();
        assert_eq!(slice[0], 2);
        assert_eq!(slice[1], 1);
        assert_eq!(slice[2], 0);
    }
    #[test]
    fn test_page_number_from_address() {
        let addr = 1024;
        let vpn = PageNumber::ceil_address(addr);
        assert_eq!(vpn.0, 1);
        let addr = 4097;
        let vpn = PageNumber::ceil_address(addr);
        assert_eq!(vpn.0, 2);
        let addr = 4096;
        let vpn = PageNumber::ceil_address(addr);
        assert_eq!(vpn.0, 1);
    }
    #[test]
    fn test_page_number_range_macro() {
        let vpn_s = vpn_f_c_range!(0, 10);
        let ppn_s = ppn_f_c_range!(0, 10);
        assert_eq!(vpn_s, VPN::new(0)..VPN::new(1));
        assert_eq!(ppn_s, PPN::new(0)..PPN::new(1));
    }
}
