#![deny(unsafe_op_in_unsafe_fn)]

pub mod assessment;
pub mod error;
pub mod github;
pub mod model;
pub mod org_audit;
pub mod policy;
pub mod render;
pub mod repo_audit;

pub const DEFAULT_CATALOG_URL: &str = "https://22-factor-apps.github.io/catalog/v1/factors.json";
pub const EDITION: &str = "2026.3";
