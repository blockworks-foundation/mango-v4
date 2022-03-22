use mango_macro::Pod;

#[derive(Copy, Clone, Pod, Default)]
#[repr(C)]
/// Stores meta information about the `Account` on chain
pub struct MetaData {
    // pub data_type: u8,
    pub version: u8,
    // pub is_initialized: bool,
    // being used by PerpMarket to store liquidity mining param
    pub extra_info: [u8; 7],
}

impl MetaData {
    pub fn new(
        // data_type: DataType,
        version: u8,
        // is_initialized: bool
    ) -> Self {
        Self {
            // data_type: data_type as u8,
            version,
            // is_initialized,
            extra_info: [0; 7],
        }
    }
    pub fn new_with_extra(
        // data_type: DataType,
        version: u8,
        // is_initialized: bool,
        extra_info: [u8; 7],
    ) -> Self {
        Self {
            // data_type: data_type as u8,
            version,
            // is_initialized,
            extra_info,
        }
    }
}
