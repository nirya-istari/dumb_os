
#[derive(Debug)]
struct RsdpDescriptor {
    // 1.0        
    pub oem_id: String,
    pub revision: u8,
    pub rsdt: Rdst
}

#[derive(Debug)]
struct Rdst {
    
}


