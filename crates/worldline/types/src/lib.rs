//! Canonical WorldLine type system.
//!
//! This crate provides the stable type entrypoint for WorldLine APIs.

pub use maple_mwl_types::*;

#[cfg(test)]
mod tests {
    use super::WorldlineId;

    #[test]
    fn worldline_id_is_available() {
        let _ = WorldlineId::ephemeral();
    }
}
