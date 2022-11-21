#![no_std]
#![allow(unused)]
extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ops::{Add, Range, Sub};

mod area;
mod entry;
mod space;
mod table;


type PhyAddr = usize;
type VirtAddr = usize;

#[derive(Copy, Clone, Debug)]
pub struct PageNumber(usize);

pub type PPN = PageNumber;
pub type VPN = PageNumber;

pub trait VPNToSlice {
    fn to_slice(&self) -> &[usize];
}


impl PageNumber {
    pub fn new(num: usize) -> Self {
        assert_eq!(num.trailing_zeros(), 12);
        Self(num)
    }
    pub fn to_address(&self) -> usize {
        self.0 << 12
    }
}

impl Sub for PageNumber {
    type Output = usize;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl  Add for PageNumber{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}

impl From<usize> for PageNumber {
    fn from(value: usize) -> Self {
        assert_eq!(num.trailing_zeros(), 12);
        Self(value)
    }
}

impl VPNToSlice for PageNumber {
    fn to_slice(&self) -> &[usize] {
        let mut slice = [0; 3];
        slice[0] = self.0 >> 27;
        slice[1] = (self.0 >> 18) & 0x1ff;
        slice[2] = (self.0 >> 9) & 0x1ff;
        unsafe { core::slice::from_raw_parts(slice.as_ptr(), slice.len()) }
    }
}