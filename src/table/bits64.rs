extern crate alloc;

use crate::addr;
use alloc::collections::BTreeMap;
use alloc::{vec, vec::Vec};
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use log::{error, trace, warn};

use crate::addr::{PhysAddr, VirtAddr, PAGE_SIZE_4K};

use crate::pte::GenericPTE;
use crate::pte::MappingFlags;
use crate::table::{PageSize, PagingError, PagingIf, PagingMetaData, PagingResult};

const ENTRY_COUNT: usize = 512;

const fn p4_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 27)) & (ENTRY_COUNT - 1)
}

const fn p3_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 18)) & (ENTRY_COUNT - 1)
}

const fn p2_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 9)) & (ENTRY_COUNT - 1)
}

const fn p1_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> 12) & (ENTRY_COUNT - 1)
}

/// A generic page table struct for 64-bit platform.
///
/// It also tracks all intermediate level tables. They will be deallocated
/// When the [`PageTable64`] itself is dropped.
pub struct PageTable64<M: PagingMetaData, PTE: GenericPTE, IF: PagingIf> {
    root_paddr: PhysAddr,
    intrm_tables: Vec<PhysAddr>,
    record: BTreeMap<VirtAddr, bool>,
    _phantom: PhantomData<(M, PTE, IF)>,
}

impl<M: PagingMetaData, PTE: GenericPTE, IF: PagingIf> PageTable64<M, PTE, IF> {
    /// Creates a new page table instance or returns the error.
    ///
    /// It will allocate a new page for the root page table.
    pub fn try_new() -> PagingResult<Self> {
        let root_paddr = Self::alloc_table()?;
        Ok(Self {
            root_paddr,
            intrm_tables: vec![root_paddr],
            record: BTreeMap::new(),
            _phantom: PhantomData,
        })
    }

    /// Returns the physical address of the root page table.
    pub const fn root_paddr(&self) -> PhysAddr {
        self.root_paddr
    }


    ///
    pub fn alloc_pages_info_mut(&mut self) ->&mut Vec<PhysAddr>{
        &mut self.intrm_tables
    }

    pub fn get_record(&self)->BTreeMap<VirtAddr, bool>{
        self.record.clone()
    }

    pub fn get_record_mut(&mut self)->&mut BTreeMap<VirtAddr, bool>{
        &mut self.record
    }

    /// Maps a virtual page to a physical frame with the given `page_size`
    /// and mapping `flags`. The physical frame will be allocated if `lazy_alloc` is true.
    /// If `lazy_alloc` is false, the physical frame is zero and user should set correct flags,because it will
    /// cause a page fault if the page is accessed.
    ///
    /// The virtual page starts with `vaddr`.If the addresses is not aligned to the page size, they will be
    /// aligned down automatically.
    ///
    /// Returns [`Err(PagingError::AlreadyMapped)`](PagingError::AlreadyMapped)
    /// if the mapping is already present.
    pub fn map_no_target(
        &mut self,
        vaddr: VirtAddr,
        page_size: PageSize,
        flags: MappingFlags,
        lazy_alloc: bool,
    ) -> PagingResult<PhysAddr> {
        let entry = self.get_entry_mut_or_create(vaddr, page_size)?;
        if !entry.is_unused() {
            return Err(PagingError::AlreadyMapped);
        }
        let mut flags = flags;
        let phy = if lazy_alloc {
            // assert!(!flags.contains(MappingFlags::V));
            flags -= MappingFlags::V;
            PhysAddr::from(0)
        } else {
            assert!(flags.contains(MappingFlags::V));
            let phy = IF::alloc_contiguous_frames(usize::from(page_size) / PAGE_SIZE_4K);
            if phy.is_none() {
                return Err(PagingError::NoMemory);
            }
            phy.unwrap()
        };
        *entry = GenericPTE::new_page(phy, flags, page_size.is_huge());
        self.record.insert(vaddr, true);
        Ok(phy)
    }

    ///
    pub fn validate(&mut self, vaddr: VirtAddr, flags:MappingFlags) -> PagingResult {
        let (entry, page_size) = self.get_entry_mut(vaddr)?;
        if entry.is_unused() {
            return Err(PagingError::NotMapped);
        }
        if entry.is_present() {
            return Err(PagingError::AlreadyValid);
        }
        let phy = IF::alloc_contiguous_frames(usize::from(page_size) / PAGE_SIZE_4K);
        if phy.is_none() {
            return Err(PagingError::NoMemory);
        }
        let phy = phy.unwrap();
        *entry = GenericPTE::new_page(phy, flags, page_size.is_huge());
        Ok(())
    }

    /// if re_alloc, user should remove old info about phy in intrm_tables
    pub fn modify_pte_flags(&mut self,vaddr: VirtAddr,flags:MappingFlags,re_alloc:bool) -> PagingResult<Option<PhysAddr>>{
        let (phy,_,page_size) = self.query(vaddr)?;
        let phy = if re_alloc{
            let phy = IF::alloc_contiguous_frames(usize::from(page_size) / PAGE_SIZE_4K);
            if phy.is_none() {
                return Err(PagingError::NoMemory);
            }
            let phy = phy.unwrap();
            phy
        }else {
            phy
        };
        let (pte,page_size) = self.get_entry_mut(vaddr)?;
        *pte =  GenericPTE::new_page(phy, flags, page_size.is_huge());
        if re_alloc{
            Ok(Some(phy))
        }else {
            Ok(None)
        }
    }


    /// Maps a virtual page to a physical frame with the given `page_size`
    /// and mapping `flags`.
    ///
    /// The virtual page starts with `vaddr`, amd the physical frame starts with
    /// `target`. If the addresses is not aligned to the page size, they will be
    /// aligned down automatically.
    ///
    /// Returns [`Err(PagingError::AlreadyMapped)`](PagingError::AlreadyMapped)
    /// if the mapping is already present.
    pub fn map(
        &mut self,
        vaddr: VirtAddr,
        target: PhysAddr,
        page_size: PageSize,
        flags: MappingFlags,
    ) -> PagingResult {
        let entry = self.get_entry_mut_or_create(vaddr, page_size)?;
        if !entry.is_unused() {
            return Err(PagingError::AlreadyMapped);
        }
        trace!("map {:x} to {:x}, [{:?}]", vaddr, target,page_size);
        *entry = GenericPTE::new_page(target.align_down(page_size), flags, page_size.is_huge());
        self.record.insert(vaddr, false);
        Ok(())
    }

    /// Unmaps the mapping starts with `vaddr`.
    ///
    /// Returns [`Err(PagingError::NotMapped)`](PagingError::NotMapped) if the
    /// mapping is not present.
    pub fn unmap(&mut self, vaddr: VirtAddr) -> PagingResult<(PhysAddr, PageSize)> {
        let (entry, size) = self.get_entry_mut(vaddr)?;
        if entry.is_unused() {
            return Err(PagingError::NotMapped);
        }
        let paddr = entry.paddr();
        let flags = entry.flags();
        entry.clear();
        // dealloc the physical frame
        let (v,target) = self.record.iter().find(|(&v,_)|{
            v==vaddr
        }).map(|(v,t)|{(v.clone(),t.clone())}).unwrap();
        if target&&flags.contains(MappingFlags::V){
            for i in 0..usize::from(size) / PAGE_SIZE_4K {
                let paddr = paddr + i * PAGE_SIZE_4K;
                IF::dealloc_frame(paddr);
            }
        }
        self.record.remove(&vaddr);
        Ok((paddr, size))
    }

    /// Query the result of the mapping starts with `vaddr`.
    ///
    /// Returns the physical address of the target frame, mapping flags, and
    /// the page size.
    ///
    /// Returns [`Err(PagingError::NotMapped)`](PagingError::NotMapped) if the
    /// mapping is not present.
    pub fn query(&self, vaddr: VirtAddr) -> PagingResult<(PhysAddr, MappingFlags, PageSize)> {
        let (entry, size) = self.get_entry_mut(vaddr)?;
        if entry.is_unused() {
            return Err(PagingError::NotMapped);
        }
        let off = vaddr.align_offset(size);
        Ok((entry.paddr() + off, entry.flags(), size))
    }

    pub fn map_region_no_target(
        &mut self,
        vaddr: VirtAddr,
        size: usize,
        flags: MappingFlags,
        allow_huge: bool,
        lazy_alloc: bool,
    ) -> PagingResult<MapRegionIter<M, PTE, IF>> {
        if !vaddr.is_aligned(PageSize::Size4K) || !addr::is_aligned(size, PageSize::Size4K.into()) {
            return Err(PagingError::NotAligned);
        }
        trace!(
            "map_region({:#x}): [{:#x}, {:#x}) -> [?, ?) {:?}",
            self.root_paddr(),
            vaddr,
            vaddr + size,
            flags,
        );
        let old_size = size;
        let old_vaddr = vaddr;
        let mut vaddr = vaddr;
        let mut size = size;
        while size > 0 {
            let page_size = if allow_huge {
                if vaddr.is_aligned(PageSize::Size1G) && size >= PageSize::Size1G as usize {
                    PageSize::Size1G
                } else if vaddr.is_aligned(PageSize::Size2M) && size >= PageSize::Size2M as usize {
                    PageSize::Size2M
                } else {
                    PageSize::Size4K
                }
            } else {
                PageSize::Size4K
            };
            self.map_no_target(vaddr, page_size, flags, lazy_alloc)
                .inspect_err(|e| {
                    error!(
                        "failed to map page: {:#x?}({:?}) -> ?, {:?}",
                        vaddr, page_size, e
                    )
                })?;
            vaddr += page_size as usize;
            size -= page_size as usize;
        }
        let iter = MapRegionIter {
            table: self,
            vaddr: old_vaddr,
            size: old_size,
        };
        Ok(iter)
    }

    /// Map a contiguous virtual memory region to a contiguous physical memory
    /// region with the given mapping `flags`.
    ///
    /// The virtual and physical memory regions start with `vaddr` and `paddr`
    /// respectively. The region size is `size`. The addresses and `size` must
    /// be aligned to 4K, otherwise it will return [`Err(PagingError::NotAligned)`].
    ///
    /// When `allow_huge` is true, it will try to map the region with huge pages
    /// if possible. Otherwise, it will map the region with 4K pages.
    ///
    /// [`Err(PagingError::NotAligned)`]: PagingError::NotAligned
    pub fn map_region(
        &mut self,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        size: usize,
        flags: MappingFlags,
        allow_huge: bool,
    ) -> PagingResult {
        if !vaddr.is_aligned(PageSize::Size4K)
            || !paddr.is_aligned(PageSize::Size4K)
            || !addr::is_aligned(size, PageSize::Size4K.into())
        {
            return Err(PagingError::NotAligned);
        }
        trace!(
            "map_region({:#x}): [{:#x}, {:#x}) -> [{:#x}, {:#x}) {:?}",
            self.root_paddr(),
            vaddr,
            vaddr + size,
            paddr,
            paddr + size,
            flags,
        );
        let mut vaddr = vaddr;
        let mut paddr = paddr;
        let mut size = size;
        while size > 0 {
            let page_size = if allow_huge {
                if vaddr.is_aligned(PageSize::Size1G)
                    && paddr.is_aligned(PageSize::Size1G)
                    && size >= PageSize::Size1G as usize
                {
                    PageSize::Size1G
                } else if vaddr.is_aligned(PageSize::Size2M)
                    && paddr.is_aligned(PageSize::Size2M)
                    && size >= PageSize::Size2M as usize
                {
                    PageSize::Size2M
                } else {
                    PageSize::Size4K
                }
            } else {
                PageSize::Size4K
            };
            self.map(vaddr, paddr, page_size, flags).inspect_err(|e| {
                error!(
                    "failed to map page: {:#x?}({:?}) -> {:#x?}, {:?}",
                    vaddr, page_size, paddr, e
                )
            })?;
            vaddr += page_size as usize;
            paddr += page_size as usize;
            size -= page_size as usize;
        }
        Ok(())
    }

    /// Unmap a contiguous virtual memory region.
    ///
    /// The region must be mapped before using [`PageTable64::map_region`], or
    /// unexpected behaviors may occur.
    pub fn unmap_region(&mut self, vaddr: VirtAddr, size: usize) -> PagingResult {
        trace!(
            "unmap_region({:#x}) [{:#x}, {:#x})",
            self.root_paddr(),
            vaddr,
            vaddr + size,
        );
        let mut vaddr = vaddr;
        let mut size = size;
        while size > 0 {
            let (_, page_size) = self
                .unmap(vaddr)
                .inspect_err(|e| error!("failed to unmap page: {:#x?}, {:?}", vaddr, e))?;
            assert!(vaddr.is_aligned(page_size));
            assert!(page_size as usize <= size);
            vaddr += page_size as usize;
            size -= page_size as usize;
        }
        Ok(())
    }

    /// Walk the page table recursively.
    ///
    /// When reaching the leaf page table, call `func` on the current page table
    /// entry. The max number of enumerations in one table is limited by `limit`.
    ///
    /// The arguments of `func` are:
    /// - Current level (starts with `0`): `usize`
    /// - The index of the entry in the current-level table: `usize`
    /// - The virtual address that is mapped to the entry: [`VirtAddr`]
    /// - The reference of the entry: [`&PTE`](GenericPTE)
    pub fn walk<F>(&self, limit: usize, func: &F) -> PagingResult
    where
        F: Fn(usize, usize, VirtAddr, &PTE),
    {
        self.walk_recursive(
            self.table_of(self.root_paddr()),
            0,
            VirtAddr::from(0),
            limit,
            func,
        )
    }

    /// release the page table before drop
    pub fn release(&mut self){
        for (v_addr,&target) in &self.record{
            let (phy,flags,page_size) = self.query(*v_addr).inspect_err(|x|{
                panic!("drop page table error: {:?}, vaddr: {:?}",x,v_addr);
            }).unwrap();
            if flags.contains(MappingFlags::V) & target{
                for i in 0..usize::from(page_size) / PAGE_SIZE_4K {
                    let paddr = phy + i * PAGE_SIZE_4K;
                    IF::dealloc_frame(paddr);
                }
            }
        }
        for frame in &self.intrm_tables {
            IF::dealloc_frame(*frame);
        }
        self.intrm_tables.clear();
        self.record.clear();
    }
}

// Private implements.
impl<M: PagingMetaData, PTE: GenericPTE, IF: PagingIf> PageTable64<M, PTE, IF> {
    fn alloc_table() -> PagingResult<PhysAddr> {
        if let Some(paddr) = IF::alloc_frame() {
            let ptr = IF::phys_to_virt(paddr).as_mut_ptr();
            unsafe { core::ptr::write_bytes(ptr, 0, PAGE_SIZE_4K) };
            Ok(paddr)
        } else {
            Err(PagingError::NoMemory)
        }
    }

    fn table_of<'a>(&self, paddr: PhysAddr) -> &'a [PTE] {
        let ptr = IF::phys_to_virt(paddr).as_ptr() as _;
        unsafe { core::slice::from_raw_parts(ptr, ENTRY_COUNT) }
    }

    fn table_of_mut<'a>(&self, paddr: PhysAddr) -> &'a mut [PTE] {
        let ptr = IF::phys_to_virt(paddr).as_mut_ptr() as _;
        unsafe { core::slice::from_raw_parts_mut(ptr, ENTRY_COUNT) }
    }

    fn next_table_mut<'a>(&self, entry: &PTE) -> PagingResult<&'a mut [PTE]> {
        if !entry.is_present() {
            Err(PagingError::NotMapped)
        } else if entry.is_huge() {
            Err(PagingError::MappedToHugePage)
        } else {
            Ok(self.table_of_mut(entry.paddr()))
        }
    }

    fn next_table_mut_or_create<'a>(&mut self, entry: &mut PTE) -> PagingResult<&'a mut [PTE]> {
        if entry.is_unused() {
            let paddr = Self::alloc_table()?;
            self.intrm_tables.push(paddr);
            *entry = GenericPTE::new_table(paddr);
            Ok(self.table_of_mut(paddr))
        } else {
            self.next_table_mut(entry)
        }
    }

    fn get_entry_mut(&self, vaddr: VirtAddr) -> PagingResult<(&mut PTE, PageSize)> {
        let p3 = if M::LEVELS == 3 {
            self.table_of_mut(self.root_paddr())
        } else if M::LEVELS == 4 {
            let p4 = self.table_of_mut(self.root_paddr());
            let p4e = &mut p4[p4_index(vaddr)];
            self.next_table_mut(p4e)?
        } else {
            unreachable!()
        };
        let p3e = &mut p3[p3_index(vaddr)];
        if p3e.is_huge() {
            return Ok((p3e, PageSize::Size1G));
        }

        let p2 = self.next_table_mut(p3e)?;
        let p2e = &mut p2[p2_index(vaddr)];
        if p2e.is_huge() {
            return Ok((p2e, PageSize::Size2M));
        }

        let p1 = self.next_table_mut(p2e)?;
        let p1e = &mut p1[p1_index(vaddr)];
        Ok((p1e, PageSize::Size4K))
    }

    fn get_entry_mut_or_create(
        &mut self,
        vaddr: VirtAddr,
        page_size: PageSize,
    ) -> PagingResult<&mut PTE> {
        let p3 = if M::LEVELS == 3 {
            self.table_of_mut(self.root_paddr())
        } else if M::LEVELS == 4 {
            let p4 = self.table_of_mut(self.root_paddr());
            let p4e = &mut p4[p4_index(vaddr)];
            self.next_table_mut_or_create(p4e)?
        } else {
            unreachable!()
        };
        let p3e = &mut p3[p3_index(vaddr)];
        if page_size == PageSize::Size1G {
            return Ok(p3e);
        }

        let p2 = self.next_table_mut_or_create(p3e)?;
        let p2e = &mut p2[p2_index(vaddr)];
        if page_size == PageSize::Size2M {
            return Ok(p2e);
        }

        let p1 = self.next_table_mut_or_create(p2e)?;
        let p1e = &mut p1[p1_index(vaddr)];
        Ok(p1e)
    }

    fn walk_recursive<F>(
        &self,
        table: &[PTE],
        level: usize,
        start_vaddr: VirtAddr,
        limit: usize,
        func: &F,
    ) -> PagingResult
    where
        F: Fn(usize, usize, VirtAddr, &PTE),
    {
        let mut n = 0;
        for (i, entry) in table.iter().enumerate() {
            let vaddr = start_vaddr + (i << (12 + (M::LEVELS - 1 - level) * 9));
            if entry.is_present() {
                func(level, i, vaddr, entry);
                if level < M::LEVELS - 1 && !entry.is_huge() {
                    let table_entry = self.next_table_mut(entry)?;
                    self.walk_recursive(table_entry, level + 1, vaddr, limit, func)?;
                }
                n += 1;
                if n >= limit {
                    break;
                }
            }
        }
        Ok(())
    }
}

impl<M: PagingMetaData, PTE: GenericPTE, IF: PagingIf> Drop for PageTable64<M, PTE, IF> {
    fn drop(&mut self) {
        for (v_addr,target) in &self.record{
            let (phy,flags,page_size) = self.query(*v_addr).inspect_err(|x|{
                panic!("drop page table error: {:?}, vaddr: {:?}",x,v_addr);
            }).unwrap();
            if flags.contains(MappingFlags::V) & *target{
                for i in 0..usize::from(page_size) / PAGE_SIZE_4K {
                    let paddr = phy + i * PAGE_SIZE_4K;
                    IF::dealloc_frame(paddr);
                }
            }
        }
        for frame in &self.intrm_tables {
            IF::dealloc_frame(*frame);
        }
    }
}

pub struct MapRegionIter<'a, M: PagingMetaData, PTE: GenericPTE, IF: PagingIf> {
    table: &'a PageTable64<M, PTE, IF>,
    vaddr: VirtAddr,
    size: usize,
}

impl<'a, M, PTE, IF> Iterator for MapRegionIter<'a, M, PTE, IF>
where
    M: PagingMetaData,
    PTE: GenericPTE,
    IF: PagingIf,
{
    type Item = (VirtAddr, PhysAddr, PageSize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.size == 0 {
            return None;
        }
        let (paddr, _, page_size) = self.table.query(self.vaddr).unwrap();
        let size = page_size.into();
        self.vaddr += size;
        self.size -= size;
        Some((self.vaddr - size, paddr, page_size))
    }
}



impl<M: PagingMetaData, PTE: GenericPTE, IF: PagingIf> Debug for PageTable64<M, PTE, IF> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "PageTable64 {{")?;
        for (vaddr,_x) in &self.record {
            let (_target, flags, page_size) = self.query(*vaddr).unwrap();
            write!(f, "\n  {:x?} -> {:x?} ({:?})", vaddr, flags, page_size)?;
        }
        write!(f, "\n}}")
    }
}
