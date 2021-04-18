use alloc::prelude::v1::*;

#[derive(Debug)]
struct Bus {
    id: u8,
    port_base: u16,
    selected: Drive
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Drive {
    Primary, Secondary
}

#[derive(Debug)]
struct AtaState {
    busses: Vec<Bus>,
}

pub async fn ata_main() {
     
}
