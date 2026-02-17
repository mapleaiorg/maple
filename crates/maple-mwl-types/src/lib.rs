//! Compatibility wrapper for legacy `maple-mwl-types` imports.
//!
//! New code should depend on `worldline-types`.

pub use worldline_types::*;

#[cfg(test)]
mod tests {
    use super::WorldlineId;

    #[test]
    fn wrapper_exports_worldline_id() {
        let _ = WorldlineId::ephemeral();
    }
}
