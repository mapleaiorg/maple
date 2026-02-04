//! Generated protobuf modules.
//!
//! The generated artifact is checked in so this workspace builds in restricted/offline
//! environments without requiring a build-time codegen dependency download.

pub mod ibank {
    pub mod v1 {
        pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("generated/ibank_descriptor.bin");
        include!("generated/ibank.v1.rs");
    }
}
