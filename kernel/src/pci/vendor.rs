
struct Vendor {
    id: u16,
    name: &'static str,
    devices: &'static [Device],
    sub_vendors: &'static [Vendor],
}

struct Device {
    id: u16,
    name: &'static str,
}


struct VendorId(u16);

struct DeviceId(u16, u16);

