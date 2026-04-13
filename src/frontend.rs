//! Helpers for loading compiled frontend assets and opening built HTML documents.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use actix_files::NamedFile;
use serde::Deserialize;
use thiserror::Error;

/// Root directory for built frontend artifacts emitted by Vite.
pub const FRONTEND_DIST_DIR: &str = "assets/dist";

/// Relative path of the Vite manifest inside [`FRONTEND_DIST_DIR`].
pub const FRONTEND_MANIFEST_FILE: &str = "manifest.json";

/// Built HTML document that will eventually back `GET /`.
pub const FRONTEND_INDEX_DOCUMENT: &str = "app/index.html";

/// Built HTML document backing `GET /na`.
pub const FRONTEND_NO_ACCESS_DOCUMENT: &str = "app/no-access.html";

/// Built HTML document backing `GET /order/{order_id}`.
pub const FRONTEND_ORDER_DOCUMENT: &str = "app/order.html";

/// Built HTML document backing `GET /products`.
pub const FRONTEND_PRODUCTS_DOCUMENT: &str = "app/products.html";

/// Built HTML document backing `GET /categories`.
pub const FRONTEND_CATEGORIES_DOCUMENT: &str = "app/categories.html";

/// Built HTML document backing `GET /tags`.
pub const FRONTEND_TAGS_DOCUMENT: &str = "app/tags.html";

/// Built HTML document backing `GET /price-levels`.
pub const FRONTEND_PRICE_LEVELS_DOCUMENT: &str = "app/price-levels.html";

/// Built HTML document backing `GET /vendors`.
pub const FRONTEND_VENDORS_DOCUMENT: &str = "app/vendors.html";

/// Minimal subset of a Vite manifest entry needed by the backend.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FrontendManifestEntry {
    pub file: String,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub imports: Vec<String>,
}

/// Vite manifest keyed by source entry name such as `app/index.html`.
pub type FrontendManifest = BTreeMap<String, FrontendManifestEntry>;

/// Errors raised while loading frontend assets.
#[derive(Debug, Error)]
pub enum FrontendAssetError {
    #[error("failed to read frontend manifest: {0}")]
    ManifestRead(std::io::Error),
    #[error("failed to parse frontend manifest: {0}")]
    ManifestParse(serde_json::Error),
    #[error("frontend manifest entry not found: {0}")]
    MissingEntry(String),
    #[error("failed to open frontend document: {0}")]
    Read(#[from] std::io::Error),
}

/// Absolute filesystem path for the Vite manifest.
pub fn frontend_manifest_path() -> PathBuf {
    Path::new(FRONTEND_DIST_DIR).join(FRONTEND_MANIFEST_FILE)
}

/// Absolute filesystem path for a built frontend HTML document.
pub fn frontend_document_path(document: &str) -> PathBuf {
    Path::new(FRONTEND_DIST_DIR).join(document)
}

/// Load and parse the Vite manifest file.
pub fn load_frontend_manifest(
    path: impl AsRef<Path>,
) -> Result<FrontendManifest, FrontendAssetError> {
    let manifest_bytes = std::fs::read(path).map_err(FrontendAssetError::ManifestRead)?;
    serde_json::from_slice(&manifest_bytes).map_err(FrontendAssetError::ManifestParse)
}

/// Resolve a named Vite entry such as `app/index.html`.
pub fn resolve_frontend_entry<'a>(
    manifest: &'a FrontendManifest,
    entry_name: &str,
) -> Result<&'a FrontendManifestEntry, FrontendAssetError> {
    manifest
        .get(entry_name)
        .ok_or_else(|| FrontendAssetError::MissingEntry(entry_name.to_owned()))
}

/// Open a Vite-built HTML document for a future React-owned route.
pub async fn open_frontend_html(path: impl AsRef<Path>) -> Result<NamedFile, FrontendAssetError> {
    let file = NamedFile::open_async(path).await?;
    Ok(file.use_last_modified(true).prefer_utf8(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_and_resolves_manifest_entry() {
        let dir = tempdir().expect("tempdir should be created");
        let manifest_path = dir.path().join("manifest.json");

        std::fs::write(
            &manifest_path,
            r#"{
  "app/index.html": {
    "file": "entries/app/index.html-abc123.js",
    "css": ["styles/app/index.html-abc123.css"],
    "imports": ["_shared-vendor-xyz.js"],
    "isEntry": true
  }
}"#,
        )
        .expect("manifest should be written");

        let manifest = load_frontend_manifest(&manifest_path).expect("manifest should parse");
        let entry =
            resolve_frontend_entry(&manifest, "app/index.html").expect("entry should exist");

        assert_eq!(entry.file, "entries/app/index.html-abc123.js");
        assert_eq!(entry.css, vec!["styles/app/index.html-abc123.css"]);
        assert_eq!(entry.imports, vec!["_shared-vendor-xyz.js"]);
    }

    #[test]
    fn missing_entry_returns_error() {
        let manifest = FrontendManifest::new();

        let error = resolve_frontend_entry(&manifest, "app/index.html")
            .expect_err("missing entry should return an error");

        assert!(matches!(
            error,
            FrontendAssetError::MissingEntry(name) if name == "app/index.html"
        ));
    }

    #[test]
    fn can_open_existing_file() {
        let dir = tempdir().expect("tempdir should be created");
        let html_path = dir.path().join("index.html");
        std::fs::write(&html_path, "<!doctype html><title>ok</title>")
            .expect("html file should be written");

        let result = actix_web::rt::System::new().block_on(open_frontend_html(&html_path));
        assert!(result.is_ok());
    }

    #[test]
    fn missing_document_returns_read_error() {
        let error = actix_web::rt::System::new()
            .block_on(open_frontend_html("assets/dist/does-not-exist.html"))
            .expect_err("missing file should return an error");

        assert!(matches!(error, FrontendAssetError::Read(_)));
    }
}
