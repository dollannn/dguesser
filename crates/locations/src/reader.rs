//! Range reader trait and implementations for accessing pack data.
//!
//! This module provides an abstraction over storage backends:
//! - `HttpReader`: For R2/S3 via HTTP Range requests
//! - `FileReader`: For local development with file-based packs

use std::path::{Path, PathBuf};

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::error::LocationPackError;
use crate::index::CountryIndex;
use crate::manifest::Manifest;

// Re-export async_trait for implementors
pub use async_trait::async_trait;

/// Trait for reading data from pack storage with Range support.
#[async_trait]
pub trait RangeReader: Send + Sync {
    /// Read the manifest file.
    async fn read_manifest(&self) -> Result<Manifest, LocationPackError>;

    /// Read a country's index file.
    async fn read_country_index(&self, country: &str) -> Result<CountryIndex, LocationPackError>;

    /// Read a byte range from a pack file.
    ///
    /// # Arguments
    /// * `country` - Country code (e.g., "US")
    /// * `pack_name` - Pack file name (e.g., "US_B4_S0.pack")
    /// * `offset` - Starting byte offset
    /// * `length` - Number of bytes to read
    async fn read_pack_range(
        &self,
        country: &str,
        pack_name: &str,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, LocationPackError>;
}

/// HTTP-based reader for R2/S3 compatible storage.
pub struct HttpReader {
    client: reqwest::Client,
    base_url: String,
    version: String,
}

impl HttpReader {
    /// Create a new HTTP reader.
    ///
    /// # Arguments
    /// * `base_url` - Base URL for the R2 bucket (e.g., "https://bucket.r2.cloudflarestorage.com")
    /// * `version` - Dataset version (e.g., "v2026-01")
    pub fn new(base_url: &str, version: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            version: version.to_string(),
        }
    }

    /// Build the full URL for a path.
    fn url(&self, path: &str) -> String {
        format!("{}/{}/{}", self.base_url, self.version, path)
    }
}

#[async_trait]
impl RangeReader for HttpReader {
    async fn read_manifest(&self) -> Result<Manifest, LocationPackError> {
        let url = self.url("manifest.json");
        tracing::debug!(url = %url, "Fetching manifest");

        let response = self.client.get(&url).send().await?.error_for_status()?;
        let manifest: Manifest = response.json().await?;

        Ok(manifest)
    }

    async fn read_country_index(&self, country: &str) -> Result<CountryIndex, LocationPackError> {
        let url = self.url(&format!("countries/{}/index.json", country));
        tracing::debug!(url = %url, "Fetching country index");

        let response = self.client.get(&url).send().await?.error_for_status().map_err(|e| {
            if e.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                LocationPackError::CountryNotFound(country.to_string())
            } else {
                LocationPackError::Http(e)
            }
        })?;

        let index: CountryIndex = response.json().await?;
        Ok(index)
    }

    async fn read_pack_range(
        &self,
        country: &str,
        pack_name: &str,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, LocationPackError> {
        let url = self.url(&format!("countries/{}/{}", country, pack_name));
        let range_end = offset + length - 1;
        let range_header = format!("bytes={}-{}", offset, range_end);

        tracing::debug!(url = %url, range = %range_header, "Fetching pack range");

        let response = self
            .client
            .get(&url)
            .header(reqwest::header::RANGE, range_header)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.bytes().await?)
    }
}

/// File-based reader for local development.
pub struct FileReader {
    base_path: PathBuf,
    version: String,
}

impl FileReader {
    /// Create a new file reader.
    ///
    /// # Arguments
    /// * `base_path` - Base directory containing version subdirectories
    /// * `version` - Dataset version (e.g., "v2026-01")
    pub fn new(base_path: impl AsRef<Path>, version: &str) -> Self {
        Self { base_path: base_path.as_ref().to_path_buf(), version: version.to_string() }
    }

    /// Get the path for a file within the version directory.
    fn path(&self, relative: &str) -> PathBuf {
        self.base_path.join(&self.version).join(relative)
    }
}

#[async_trait]
impl RangeReader for FileReader {
    async fn read_manifest(&self) -> Result<Manifest, LocationPackError> {
        let path = self.path("manifest.json");
        tracing::debug!(path = ?path, "Reading manifest");

        let content = tokio::fs::read_to_string(&path).await?;
        let manifest: Manifest = serde_json::from_str(&content)?;

        Ok(manifest)
    }

    async fn read_country_index(&self, country: &str) -> Result<CountryIndex, LocationPackError> {
        let path = self.path(&format!("countries/{}/index.json", country));
        tracing::debug!(path = ?path, "Reading country index");

        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LocationPackError::CountryNotFound(country.to_string())
            } else {
                LocationPackError::Io(e)
            }
        })?;

        let index: CountryIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    async fn read_pack_range(
        &self,
        country: &str,
        pack_name: &str,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, LocationPackError> {
        let path = self.path(&format!("countries/{}/{}", country, pack_name));
        tracing::debug!(path = ?path, offset = offset, length = length, "Reading pack range");

        let mut file = tokio::fs::File::open(&path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;

        let mut buffer = vec![0u8; length as usize];
        file.read_exact(&mut buffer).await?;

        Ok(Bytes::from(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_files() -> TempDir {
        let dir = TempDir::new().unwrap();
        let version_dir = dir.path().join("v2026-01");
        let country_dir = version_dir.join("countries/US");
        tokio::fs::create_dir_all(&country_dir).await.unwrap();

        // Write manifest
        let manifest = Manifest::new("v2026-01");
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        tokio::fs::write(version_dir.join("manifest.json"), manifest_json).await.unwrap();

        // Write country index
        let index = CountryIndex::new("US", "v2026-01");
        let index_json = serde_json::to_string(&index).unwrap();
        tokio::fs::write(country_dir.join("index.json"), index_json).await.unwrap();

        // Write a sample pack file
        let pack_data = vec![0u8; 1600]; // 10 records worth
        tokio::fs::write(country_dir.join("US_B4_S0.pack"), pack_data).await.unwrap();

        dir
    }

    #[tokio::test]
    async fn test_file_reader_manifest() {
        let dir = setup_test_files().await;
        let reader = FileReader::new(dir.path(), "v2026-01");

        let manifest = reader.read_manifest().await.unwrap();
        assert_eq!(manifest.version, "v2026-01");
    }

    #[tokio::test]
    async fn test_file_reader_country_index() {
        let dir = setup_test_files().await;
        let reader = FileReader::new(dir.path(), "v2026-01");

        let index = reader.read_country_index("US").await.unwrap();
        assert_eq!(index.country, "US");

        // Non-existent country should error
        let result = reader.read_country_index("ZZ").await;
        assert!(matches!(result, Err(LocationPackError::CountryNotFound(_))));
    }

    #[tokio::test]
    async fn test_file_reader_pack_range() {
        let dir = setup_test_files().await;
        let reader = FileReader::new(dir.path(), "v2026-01");

        // Read first 160 bytes (one record)
        let data = reader.read_pack_range("US", "US_B4_S0.pack", 0, 160).await.unwrap();
        assert_eq!(data.len(), 160);

        // Read second record
        let data = reader.read_pack_range("US", "US_B4_S0.pack", 160, 160).await.unwrap();
        assert_eq!(data.len(), 160);
    }
}
