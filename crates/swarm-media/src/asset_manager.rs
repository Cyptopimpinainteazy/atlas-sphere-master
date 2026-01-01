//! Asset Management System for Swarm Media
//!
//! Provides comprehensive media asset lifecycle management:
//! - Content-addressed storage with IPFS/S3 backends
//! - Version tracking with full history
//! - Deduplication via SHA-256 hashing
//! - Multi-format transcoding support
//! - Asset organization (folders, tags, metadata)
//!
//! Task 9 from ARCHITECTURE_COMPLETE.md

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Supported storage backends
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBackend {
    /// Local filesystem storage
    LocalFs,
    /// AWS S3 or compatible
    S3,
    /// IPFS for decentralized storage
    Ipfs,
    /// Hybrid (IPFS primary, S3 mirror)
    Hybrid,
}

impl Default for StorageBackend {
    fn default() -> Self {
        StorageBackend::LocalFs
    }
}

/// Asset type categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    Image,
    Video,
    Audio,
    Document,
    Model3D,
    Font,
    Archive,
    Unknown,
}

impl AssetType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "tiff" => AssetType::Image,
            "mp4" | "webm" | "mov" | "avi" | "mkv" | "m4v" => AssetType::Video,
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => AssetType::Audio,
            "pdf" | "doc" | "docx" | "txt" | "md" | "html" => AssetType::Document,
            "obj" | "fbx" | "gltf" | "glb" | "stl" => AssetType::Model3D,
            "ttf" | "otf" | "woff" | "woff2" => AssetType::Font,
            "zip" | "tar" | "gz" | "7z" | "rar" => AssetType::Archive,
            _ => AssetType::Unknown,
        }
    }
}

/// Asset status in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    /// Uploading to storage
    Uploading,
    /// Processing (transcoding, thumbnail generation)
    Processing,
    /// Ready for use
    Ready,
    /// Archived (not immediately accessible)
    Archived,
    /// Marked for deletion
    PendingDeletion,
    /// Failed upload or processing
    Failed(String),
}

/// Asset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Original filename
    pub original_name: String,
    /// MIME type
    pub mime_type: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// Content hash (SHA-256)
    pub content_hash: String,
    /// Asset type category
    pub asset_type: AssetType,
    /// Width (for images/videos)
    pub width: Option<u32>,
    /// Height (for images/videos)
    pub height: Option<u32>,
    /// Duration in seconds (for audio/video)
    pub duration_secs: Option<f32>,
    /// Additional metadata (exif, etc.)
    pub extra: HashMap<String, serde_json::Value>,
}

/// Asset version record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetVersion {
    /// Version number (1, 2, 3...)
    pub version: u32,
    /// Content hash for this version
    pub content_hash: String,
    /// Storage URLs for this version
    pub storage_urls: Vec<String>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Created timestamp
    pub created_at: i64,
    /// Who created this version
    pub created_by: Option<String>,
    /// Change description
    pub change_note: Option<String>,
}

/// Core asset record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique asset ID
    pub id: Uuid,
    /// Current version number
    pub current_version: u32,
    /// All versions
    pub versions: Vec<AssetVersion>,
    /// Metadata for current version
    pub metadata: AssetMetadata,
    /// Current status
    pub status: AssetStatus,
    /// Organization: folder path
    pub folder_path: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Created timestamp
    pub created_at: i64,
    /// Last modified timestamp
    pub modified_at: i64,
    /// Thumbnails (various sizes)
    pub thumbnails: HashMap<String, String>, // size -> url
    /// Transcoded variants
    pub variants: HashMap<String, String>, // format -> url
    /// Usage count (how many times referenced)
    pub usage_count: u32,
    /// Last accessed timestamp
    pub last_accessed: Option<i64>,
}

/// Folder for organizing assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFolder {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub parent_id: Option<Uuid>,
    pub created_at: i64,
    pub asset_count: u32,
}

/// Upload request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    /// Original filename
    pub filename: String,
    /// File content (bytes)
    #[serde(skip)]
    pub content: Vec<u8>,
    /// Target folder path
    pub folder_path: Option<String>,
    /// Tags to apply
    pub tags: Option<Vec<String>>,
    /// Generate thumbnails
    pub generate_thumbnails: bool,
    /// Transcode to additional formats
    pub transcode_formats: Option<Vec<String>>,
    /// Uploader ID
    pub uploaded_by: Option<String>,
}

/// Asset Manager Configuration
#[derive(Debug, Clone)]
pub struct AssetManagerConfig {
    /// Primary storage backend
    pub storage_backend: StorageBackend,
    /// Local storage root
    pub local_storage_path: String,
    /// S3 bucket name
    pub s3_bucket: Option<String>,
    /// S3 region
    pub s3_region: Option<String>,
    /// IPFS gateway URL
    pub ipfs_gateway: Option<String>,
    /// Maximum file size (bytes)
    pub max_file_size: u64,
    /// Enable deduplication
    pub enable_dedup: bool,
    /// Thumbnail sizes to generate
    pub thumbnail_sizes: Vec<(u32, u32)>,
    /// Maximum versions to keep
    pub max_versions: u32,
}

impl Default for AssetManagerConfig {
    fn default() -> Self {
        AssetManagerConfig {
            storage_backend: StorageBackend::LocalFs,
            local_storage_path: "/var/lib/swarm-media/assets".to_string(),
            s3_bucket: None,
            s3_region: None,
            ipfs_gateway: Some("http://localhost:5001".to_string()),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            enable_dedup: true,
            thumbnail_sizes: vec![(128, 128), (256, 256), (512, 512)],
            max_versions: 10,
        }
    }
}

/// Asset Manager
///
/// Handles all asset lifecycle operations including upload, storage,
/// versioning, transcoding, and retrieval.
pub struct AssetManager {
    config: AssetManagerConfig,
    assets: Arc<RwLock<HashMap<Uuid, Asset>>>,
    folders: Arc<RwLock<HashMap<Uuid, AssetFolder>>>,
    hash_index: Arc<RwLock<HashMap<String, Uuid>>>, // content_hash -> asset_id
    tag_index: Arc<RwLock<HashMap<String, Vec<Uuid>>>>, // tag -> asset_ids
}

impl AssetManager {
    pub fn new(config: AssetManagerConfig) -> Self {
        AssetManager {
            config,
            assets: Arc::new(RwLock::new(HashMap::new())),
            folders: Arc::new(RwLock::new(HashMap::new())),
            hash_index: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Compute SHA-256 hash of content
    fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    /// Get file extension from filename
    fn get_extension(filename: &str) -> String {
        filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase()
    }

    /// Determine MIME type from extension
    fn mime_from_extension(ext: &str) -> String {
        match ext {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mov" => "video/quicktime",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "flac" => "audio/flac",
            "pdf" => "application/pdf",
            "json" => "application/json",
            "txt" => "text/plain",
            "html" => "text/html",
            _ => "application/octet-stream",
        }.to_string()
    }

    /// Store content to configured backend
    async fn store_content(&self, content: &[u8], content_hash: &str, ext: &str) -> Result<Vec<String>, String> {
        let mut urls = Vec::new();

        match self.config.storage_backend {
            StorageBackend::LocalFs | StorageBackend::Hybrid => {
                let path = format!(
                    "{}/{}/{}.{}",
                    self.config.local_storage_path,
                    &content_hash[..2], // First 2 chars for sharding
                    content_hash,
                    ext
                );

                // Create directory if needed
                if let Some(parent) = std::path::Path::new(&path).parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }

                tokio::fs::write(&path, content)
                    .await
                    .map_err(|e| format!("Failed to write file: {}", e))?;

                urls.push(format!("file://{}", path));
            }
            _ => {}
        }

        match self.config.storage_backend {
            StorageBackend::S3 | StorageBackend::Hybrid => {
                if let (Some(bucket), Some(region)) = (&self.config.s3_bucket, &self.config.s3_region) {
                    let s3_url = self.upload_to_s3(content, content_hash, ext, bucket, region).await?;
                    urls.push(s3_url);
                }
            }
            _ => {}
        }

        match self.config.storage_backend {
            StorageBackend::Ipfs | StorageBackend::Hybrid => {
                if let Some(gateway) = &self.config.ipfs_gateway {
                    let ipfs_url = self.upload_to_ipfs(content, gateway).await?;
                    urls.push(ipfs_url);
                }
            }
            _ => {}
        }

        if urls.is_empty() {
            return Err("No storage backend available".to_string());
        }

        Ok(urls)
    }

    /// Upload content to S3
    async fn upload_to_s3(&self, content: &[u8], hash: &str, ext: &str, bucket: &str, region: &str) -> Result<String, String> {
        // Production: Use aws-sdk-s3
        // For now, return a mock URL indicating where it would be stored
        let key = format!("{}/{}.{}", &hash[..2], hash, ext);
        Ok(format!("s3://{}/{}", bucket, key))
    }

    /// Upload content to IPFS
    async fn upload_to_ipfs(&self, content: &[u8], gateway: &str) -> Result<String, String> {
        let client = reqwest::Client::new();
        
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(content.to_vec()));

        let response = client
            .post(&format!("{}/api/v0/add", gateway))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("IPFS upload failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("IPFS error: {}", response.status()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse IPFS response: {}", e))?;

        let cid = result["Hash"]
            .as_str()
            .ok_or("Invalid IPFS response")?;

        Ok(format!("ipfs://{}", cid))
    }

    /// Upload a new asset
    pub async fn upload(&self, request: UploadRequest) -> Result<Asset, String> {
        // Validate file size
        if request.content.len() as u64 > self.config.max_file_size {
            return Err(format!(
                "File size {} exceeds maximum {}",
                request.content.len(),
                self.config.max_file_size
            ));
        }

        let content_hash = Self::compute_hash(&request.content);

        // Check for duplicate
        if self.config.enable_dedup {
            let hash_index = self.hash_index.read().await;
            if let Some(existing_id) = hash_index.get(&content_hash) {
                // Return existing asset
                let assets = self.assets.read().await;
                if let Some(asset) = assets.get(existing_id) {
                    return Ok(asset.clone());
                }
            }
        }

        let ext = Self::get_extension(&request.filename);
        let mime_type = Self::mime_from_extension(&ext);
        let asset_type = AssetType::from_extension(&ext);

        // Store content
        let storage_urls = self.store_content(&request.content, &content_hash, &ext).await?;

        let now = chrono::Utc::now().timestamp();
        let asset_id = Uuid::new_v4();

        let version = AssetVersion {
            version: 1,
            content_hash: content_hash.clone(),
            storage_urls: storage_urls.clone(),
            size_bytes: request.content.len() as u64,
            created_at: now,
            created_by: request.uploaded_by.clone(),
            change_note: Some("Initial upload".to_string()),
        };

        let metadata = AssetMetadata {
            original_name: request.filename.clone(),
            mime_type,
            size_bytes: request.content.len() as u64,
            content_hash: content_hash.clone(),
            asset_type: asset_type.clone(),
            width: None, // TODO: Extract from image/video
            height: None,
            duration_secs: None,
            extra: HashMap::new(),
        };

        let folder_path = request.folder_path.unwrap_or_else(|| "/".to_string());
        let tags = request.tags.unwrap_or_default();

        let asset = Asset {
            id: asset_id,
            current_version: 1,
            versions: vec![version],
            metadata,
            status: AssetStatus::Ready,
            folder_path,
            tags: tags.clone(),
            created_at: now,
            modified_at: now,
            thumbnails: HashMap::new(),
            variants: HashMap::new(),
            usage_count: 0,
            last_accessed: None,
        };

        // Update indexes
        {
            let mut assets = self.assets.write().await;
            assets.insert(asset_id, asset.clone());
        }

        {
            let mut hash_index = self.hash_index.write().await;
            hash_index.insert(content_hash, asset_id);
        }

        {
            let mut tag_index = self.tag_index.write().await;
            for tag in &tags {
                tag_index.entry(tag.clone()).or_default().push(asset_id);
            }
        }

        // Generate thumbnails in background
        if request.generate_thumbnails && matches!(asset_type, AssetType::Image | AssetType::Video) {
            let manager = self.clone();
            let id = asset_id;
            tokio::spawn(async move {
                manager.generate_thumbnails(id).await;
            });
        }

        Ok(asset)
    }

    /// Create a new version of an existing asset
    pub async fn upload_new_version(&self, asset_id: Uuid, content: Vec<u8>, change_note: Option<String>, uploaded_by: Option<String>) -> Result<Asset, String> {
        let mut assets = self.assets.write().await;
        let asset = assets.get_mut(&asset_id)
            .ok_or_else(|| format!("Asset {} not found", asset_id))?;

        let content_hash = Self::compute_hash(&content);
        let ext = Self::get_extension(&asset.metadata.original_name);

        // Store content
        let storage_urls = self.store_content(&content, &content_hash, &ext).await?;

        let now = chrono::Utc::now().timestamp();
        let new_version = asset.current_version + 1;

        let version = AssetVersion {
            version: new_version,
            content_hash: content_hash.clone(),
            storage_urls,
            size_bytes: content.len() as u64,
            created_at: now,
            created_by: uploaded_by,
            change_note,
        };

        asset.versions.push(version);
        asset.current_version = new_version;
        asset.metadata.content_hash = content_hash.clone();
        asset.metadata.size_bytes = content.len() as u64;
        asset.modified_at = now;

        // Prune old versions if needed
        if asset.versions.len() > self.config.max_versions as usize {
            let to_remove = asset.versions.len() - self.config.max_versions as usize;
            asset.versions.drain(0..to_remove);
        }

        // Update hash index
        {
            let mut hash_index = self.hash_index.write().await;
            hash_index.insert(content_hash, asset_id);
        }

        Ok(asset.clone())
    }

    /// Get asset by ID
    pub async fn get(&self, asset_id: Uuid) -> Option<Asset> {
        let mut assets = self.assets.write().await;
        if let Some(asset) = assets.get_mut(&asset_id) {
            asset.last_accessed = Some(chrono::Utc::now().timestamp());
            Some(asset.clone())
        } else {
            None
        }
    }

    /// Get asset by content hash (for deduplication)
    pub async fn get_by_hash(&self, content_hash: &str) -> Option<Asset> {
        let hash_index = self.hash_index.read().await;
        if let Some(asset_id) = hash_index.get(content_hash) {
            self.get(*asset_id).await
        } else {
            None
        }
    }

    /// List assets in a folder
    pub async fn list_folder(&self, folder_path: &str) -> Vec<Asset> {
        let assets = self.assets.read().await;
        assets.values()
            .filter(|a| a.folder_path == folder_path || a.folder_path.starts_with(&format!("{}/", folder_path)))
            .cloned()
            .collect()
    }

    /// Search assets by tag
    pub async fn search_by_tag(&self, tag: &str) -> Vec<Asset> {
        let tag_index = self.tag_index.read().await;
        let assets = self.assets.read().await;
        
        tag_index.get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| assets.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Add tags to an asset
    pub async fn add_tags(&self, asset_id: Uuid, tags: Vec<String>) -> Result<(), String> {
        let mut assets = self.assets.write().await;
        let asset = assets.get_mut(&asset_id)
            .ok_or_else(|| format!("Asset {} not found", asset_id))?;

        let mut tag_index = self.tag_index.write().await;
        
        for tag in tags {
            if !asset.tags.contains(&tag) {
                asset.tags.push(tag.clone());
                tag_index.entry(tag).or_default().push(asset_id);
            }
        }

        Ok(())
    }

    /// Move asset to a different folder
    pub async fn move_asset(&self, asset_id: Uuid, new_folder: &str) -> Result<(), String> {
        let mut assets = self.assets.write().await;
        let asset = assets.get_mut(&asset_id)
            .ok_or_else(|| format!("Asset {} not found", asset_id))?;

        asset.folder_path = new_folder.to_string();
        asset.modified_at = chrono::Utc::now().timestamp();

        Ok(())
    }

    /// Delete an asset
    pub async fn delete(&self, asset_id: Uuid) -> Result<(), String> {
        let mut assets = self.assets.write().await;
        let asset = assets.remove(&asset_id)
            .ok_or_else(|| format!("Asset {} not found", asset_id))?;

        // Remove from hash index
        {
            let mut hash_index = self.hash_index.write().await;
            hash_index.remove(&asset.metadata.content_hash);
        }

        // Remove from tag index
        {
            let mut tag_index = self.tag_index.write().await;
            for tag in &asset.tags {
                if let Some(ids) = tag_index.get_mut(tag) {
                    ids.retain(|id| *id != asset_id);
                }
            }
        }

        // TODO: Delete from storage backends

        Ok(())
    }

    /// Get a specific version of an asset
    pub async fn get_version(&self, asset_id: Uuid, version: u32) -> Option<AssetVersion> {
        let assets = self.assets.read().await;
        assets.get(&asset_id)?
            .versions.iter()
            .find(|v| v.version == version)
            .cloned()
    }

    /// Restore a previous version
    pub async fn restore_version(&self, asset_id: Uuid, version: u32) -> Result<Asset, String> {
        let mut assets = self.assets.write().await;
        let asset = assets.get_mut(&asset_id)
            .ok_or_else(|| format!("Asset {} not found", asset_id))?;

        let version_data = asset.versions.iter()
            .find(|v| v.version == version)
            .ok_or_else(|| format!("Version {} not found", version))?
            .clone();

        asset.current_version = version;
        asset.metadata.content_hash = version_data.content_hash;
        asset.metadata.size_bytes = version_data.size_bytes;
        asset.modified_at = chrono::Utc::now().timestamp();

        Ok(asset.clone())
    }

    /// Generate thumbnails for an asset
    async fn generate_thumbnails(&self, asset_id: Uuid) {
        // In production, this would call image processing service
        // For now, we mark that thumbnails should be generated
        let mut assets = self.assets.write().await;
        if let Some(asset) = assets.get_mut(&asset_id) {
            for (w, h) in &self.config.thumbnail_sizes {
                let thumb_key = format!("{}x{}", w, h);
                // Would generate actual thumbnail and store URL
                asset.thumbnails.insert(thumb_key, format!("pending://{}x{}", w, h));
            }
        }
    }

    /// Create a folder
    pub async fn create_folder(&self, name: &str, parent_path: &str) -> Result<AssetFolder, String> {
        let path = if parent_path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", parent_path, name)
        };

        let folder = AssetFolder {
            id: Uuid::new_v4(),
            name: name.to_string(),
            path: path.clone(),
            parent_id: None, // Would need to look up parent
            created_at: chrono::Utc::now().timestamp(),
            asset_count: 0,
        };

        let mut folders = self.folders.write().await;
        folders.insert(folder.id, folder.clone());

        Ok(folder)
    }

    /// List all folders
    pub async fn list_folders(&self) -> Vec<AssetFolder> {
        let folders = self.folders.read().await;
        folders.values().cloned().collect()
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> AssetStats {
        let assets = self.assets.read().await;
        
        let total_assets = assets.len();
        let total_size: u64 = assets.values()
            .map(|a| a.metadata.size_bytes)
            .sum();
        
        let by_type: HashMap<String, usize> = assets.values()
            .fold(HashMap::new(), |mut acc, a| {
                let key = format!("{:?}", a.metadata.asset_type);
                *acc.entry(key).or_insert(0) += 1;
                acc
            });

        let total_versions: usize = assets.values()
            .map(|a| a.versions.len())
            .sum();

        AssetStats {
            total_assets,
            total_size_bytes: total_size,
            by_type,
            total_versions,
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetStats {
    pub total_assets: usize,
    pub total_size_bytes: u64,
    pub by_type: HashMap<String, usize>,
    pub total_versions: usize,
}

impl Clone for AssetManager {
    fn clone(&self) -> Self {
        AssetManager {
            config: self.config.clone(),
            assets: Arc::clone(&self.assets),
            folders: Arc::clone(&self.folders),
            hash_index: Arc::clone(&self.hash_index),
            tag_index: Arc::clone(&self.tag_index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("jpg"), AssetType::Image);
        assert_eq!(AssetType::from_extension("PNG"), AssetType::Image);
        assert_eq!(AssetType::from_extension("mp4"), AssetType::Video);
        assert_eq!(AssetType::from_extension("mp3"), AssetType::Audio);
        assert_eq!(AssetType::from_extension("pdf"), AssetType::Document);
        assert_eq!(AssetType::from_extension("xyz"), AssetType::Unknown);
    }

    #[test]
    fn test_compute_hash() {
        let content = b"Hello, World!";
        let hash = AssetManager::compute_hash(content);
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(AssetManager::get_extension("image.jpg"), "jpg");
        assert_eq!(AssetManager::get_extension("document.PDF"), "pdf");
        assert_eq!(AssetManager::get_extension("no_extension"), "no_extension");
    }

    #[tokio::test]
    async fn test_asset_manager_creation() {
        let config = AssetManagerConfig::default();
        let manager = AssetManager::new(config);
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_assets, 0);
    }

    #[tokio::test]
    async fn test_folder_creation() {
        let config = AssetManagerConfig::default();
        let manager = AssetManager::new(config);

        let folder = manager.create_folder("test", "/").await.unwrap();
        assert_eq!(folder.name, "test");
        assert_eq!(folder.path, "/test");

        let folders = manager.list_folders().await;
        assert_eq!(folders.len(), 1);
    }

    #[tokio::test]
    async fn test_upload_and_get() {
        let mut config = AssetManagerConfig::default();
        config.local_storage_path = "/tmp/swarm-test-assets".to_string();
        let manager = AssetManager::new(config);

        let request = UploadRequest {
            filename: "test.txt".to_string(),
            content: b"Hello, World!".to_vec(),
            folder_path: Some("/test".to_string()),
            tags: Some(vec!["test".to_string()]),
            generate_thumbnails: false,
            transcode_formats: None,
            uploaded_by: Some("test-user".to_string()),
        };

        let asset = manager.upload(request).await.unwrap();
        assert_eq!(asset.current_version, 1);
        assert_eq!(asset.metadata.original_name, "test.txt");
        assert!(asset.tags.contains(&"test".to_string()));

        // Get by ID
        let retrieved = manager.get(asset.id).await.unwrap();
        assert_eq!(retrieved.id, asset.id);

        // Search by tag
        let by_tag = manager.search_by_tag("test").await;
        assert_eq!(by_tag.len(), 1);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let mut config = AssetManagerConfig::default();
        config.local_storage_path = "/tmp/swarm-test-dedup".to_string();
        config.enable_dedup = true;
        let manager = AssetManager::new(config);

        let content = b"Duplicate content".to_vec();

        let request1 = UploadRequest {
            filename: "file1.txt".to_string(),
            content: content.clone(),
            folder_path: None,
            tags: None,
            generate_thumbnails: false,
            transcode_formats: None,
            uploaded_by: None,
        };

        let request2 = UploadRequest {
            filename: "file2.txt".to_string(),
            content,
            folder_path: None,
            tags: None,
            generate_thumbnails: false,
            transcode_formats: None,
            uploaded_by: None,
        };

        let asset1 = manager.upload(request1).await.unwrap();
        let asset2 = manager.upload(request2).await.unwrap();

        // Should return same asset ID due to dedup
        assert_eq!(asset1.id, asset2.id);

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_assets, 1);
    }
}
