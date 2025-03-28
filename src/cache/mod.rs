use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Cache manager for OSM data to avoid storing multiple copies of large datasets
pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    /// Create a new cache with the given directory
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        
        // Create the cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)
                .context("Failed to create cache directory")?;
        }
        
        Ok(Self { cache_dir })
    }
    
    /// Get a cached file path for the given URL
    /// Returns None if the file is not in the cache
    pub fn get_cached_file(&self, url: &str) -> Option<PathBuf> {
        let file_path = self.get_cache_path(url);
        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }
    
    /// Save data to the cache
    pub fn save_to_cache(&self, url: &str, data: &[u8]) -> Result<PathBuf> {
        let file_path = self.get_cache_path(url);
        
        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context("Failed to create parent directories for cache file")?;
            }
        }
        
        // Write the data to the file
        let mut file = File::create(&file_path)
            .context("Failed to create cache file")?;
        file.write_all(data)
            .context("Failed to write data to cache file")?;
        
        Ok(file_path)
    }
    
    /// Get the cache path for a URL
    fn get_cache_path(&self, url: &str) -> PathBuf {
        // Create a hash of the URL to use as the file name
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        
        // Get the file extension from the URL if possible
        let extension = url.split('/').last()
            .and_then(|s| s.split('.').last())
            .unwrap_or("");
        
        let file_name = if extension.is_empty() {
            hash
        } else {
            format!("{}.{}", hash, extension)
        };
        
        self.cache_dir.join(file_name)
    }
    
    /// Clear the cache
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .context("Failed to remove cache directory")?;
            fs::create_dir_all(&self.cache_dir)
                .context("Failed to recreate cache directory")?;
        }
        
        Ok(())
    }
} 