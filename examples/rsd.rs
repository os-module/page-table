use page_table::pte::MappingFlags;

fn main(){
    let flag = MappingFlags::RSD | MappingFlags::D;
    println!("{:?}",flag);
    std::time::Duration::new()
}