//! Compatibility wrapper for legacy `maple-mwl-identity` imports.
//!
//! New code should depend on `worldline-identity`.

pub use worldline_identity::*;

#[cfg(test)]
mod tests {
    use super::IdentityManager;

    #[test]
    fn wrapper_exports_identity_manager() {
        let _ = IdentityManager::new();
    }
}
