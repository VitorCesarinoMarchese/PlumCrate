pub mod operation;
pub mod transfer;
pub mod archive_import;
pub mod curseforge_client;
pub mod curseforge_browse;
pub mod error;
pub mod external_migration;
pub mod fs_scope;
pub mod lifecycle;
pub mod link_ownership;
pub mod managed_storage;
pub mod metadata_names;
pub mod mod_scan;
pub mod mod_updates;
pub mod orphan;
pub mod path_detection;
pub mod source_candidates;
pub mod source_fingerprint;
pub mod source_lookup;
pub mod source_metadata;
pub mod source_scoring;
pub mod toggle;

#[cfg(test)]
mod tests {
    use crate::error::{ErrorCode, ManagerError};

    #[test]
    fn known_error_codes_are_stable() {
        assert_eq!(ErrorCode::InvalidPath.as_str(), "INVALID_PATH");
        assert_eq!(ErrorCode::PathCollision.as_str(), "PATH_COLLISION");
        assert_eq!(ErrorCode::SourceRateLimited.as_str(), "SOURCE_RATE_LIMITED");
    }

    #[test]
    fn manager_error_serializes_contract_shape() {
        let err = ManagerError::new(ErrorCode::InvalidPath, "Invalid path");
        let json = serde_json::to_value(err).expect("serialize");

        assert_eq!(json["code"], "INVALID_PATH");
        assert_eq!(json["message"], "Invalid path");
    }
}
