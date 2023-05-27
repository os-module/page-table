use page_table::{AddressSpace, Area, AreaPermission, PageManager, PPN, VPN};
use std::alloc::{alloc, dealloc, Layout};
use std::sync::Arc;

fn main() {
    let vpn = VPN::new(0x1000);
    // 新建一个地址空间
    let mut address_space = AddressSpace::new(Arc::new(PageAllocator));
    // 新建一个map_area
    let kernel = VPN::new(0)..VPN::new(100);
    let map_area = Area::new(kernel, None, AreaPermission::R | AreaPermission::X);
    // 将map_area映射到地址空间
    address_space.push(map_area);
    let other_area = VPN::new(10000)..VPN::new(10100);
    let map_area = Area::new(other_area, None, AreaPermission::R | AreaPermission::W);
    address_space.push(map_area);
    // 删除某个映射
    println!("{:?}", address_space);
    let range = VPN::floor_address(1)..VPN::ceil_address(4096);
    for vpn in range {
        println!("{:?}", vpn);
    }
}

struct PageAllocator;
impl PageManager for PageAllocator {
    fn alloc(&self) -> Option<PPN> {
        let addr = unsafe { alloc(Layout::from_size_align(4096, 4096).unwrap()) };
        Some(PPN::ceil_address(addr as usize))
    }
    fn dealloc(&self, ppn: PPN) {
        unsafe {
            dealloc(
                ppn.to_address() as *mut u8,
                Layout::from_size_align(4096, 4096).unwrap(),
            )
        };
    }
}
