//! WorldLine identity and continuity services.

pub use maple_mwl_identity::*;
pub use worldline_types as types;

#[cfg(test)]
mod tests {
    use super::IdentityManager;

    #[test]
    fn identity_manager_is_available() {
        let _ = IdentityManager::new();
    }
}
