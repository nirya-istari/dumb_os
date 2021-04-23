use futures::Future;

pub mod ata;

pub fn disk_main() -> impl Future<Output = ()> {
    ata::ata_main()
}