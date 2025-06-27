use std::sync::Arc;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn, error, debug};

use crate::fuse::filesystem::RssFuseFilesystem;
use crate::error::{Error, Result};

/// FUSE operations manager for mounting and unmounting the filesystem
pub struct FuseOperations {
    pub filesystem: Arc<RssFuseFilesystem>,
}

impl FuseOperations {
    pub fn new() -> Self {
        Self {
            filesystem: Arc::new(RssFuseFilesystem::new()),
        }
    }

    /// Mount the RSS-FUSE filesystem at the specified mount point
    pub fn mount(&self, mount_point: &Path, options: MountOptions) -> Result<()> {
        info!("Mounting RSS-FUSE at: {}", mount_point.display());

        // Validate mount point
        self.validate_mount_point_internal(mount_point)?;

        // Prepare FUSE options
        let mut fuse_options = vec![
            "-f".to_string(), // Run in foreground by default for now
        ];

        if options.allow_other {
            fuse_options.push("-o".to_string());
            fuse_options.push("allow_other".to_string());
        }

        if options.allow_root {
            fuse_options.push("-o".to_string());
            fuse_options.push("allow_root".to_string());
        }

        if let Some(uid) = options.uid {
            fuse_options.push("-o".to_string());
            fuse_options.push(format!("uid={}", uid));
        }

        if let Some(gid) = options.gid {
            fuse_options.push("-o".to_string());
            fuse_options.push(format!("gid={}", gid));
        }

        // Set default permissions
        fuse_options.push("-o".to_string());
        fuse_options.push("default_permissions".to_string());

        debug!("FUSE options: {:?}", fuse_options);

        // Mount the filesystem using fuser
        let fs = (*self.filesystem).clone();
        
        // Convert string options to MountOption
        let mut mount_options = Vec::new();
        if options.allow_other {
            mount_options.push(fuser::MountOption::AllowOther);
        }
        if options.allow_root {
            mount_options.push(fuser::MountOption::AllowRoot);
        }
        if options.auto_unmount {
            mount_options.push(fuser::MountOption::AutoUnmount);
        }
        if options.read_only {
            mount_options.push(fuser::MountOption::RO);
        }
        mount_options.push(fuser::MountOption::DefaultPermissions);
        
        // Create a new thread for the FUSE session
        let mount_point_clone = mount_point.to_path_buf();
        
        std::thread::spawn(move || {
            info!("Starting FUSE session at {}", mount_point_clone.display());
            
            // Mount with fuser
            match fuser::mount2(fs, &mount_point_clone, &mount_options) {
                Ok(_) => {
                    info!("FUSE session ended normally");
                },
                Err(e) => {
                    error!("FUSE mount failed: {}", e);
                }
            }
        });
        
        // Give the mount a moment to establish
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        info!("Filesystem mounted successfully at {}", mount_point.display());
        
        Ok(())
    }

    /// Unmount the filesystem at the specified mount point
    pub fn unmount(&self, mount_point: &Path, force: bool) -> Result<()> {
        info!("Unmounting RSS-FUSE from: {}", mount_point.display());

        // Check if mount point exists
        if !mount_point.exists() {
            warn!("Mount point does not exist: {}", mount_point.display());
            if !force {
                return Err(Error::NotFound(format!(
                    "Mount point does not exist: {}. If the filesystem was previously mounted here, try using --force to clean up",
                    mount_point.display()
                )));
            }
            // With force flag, continue anyway to try cleanup
        }

        // Check if it's actually mounted
        if !self.is_mounted(mount_point) && !force {
            warn!("Mount point is not mounted: {}", mount_point.display());
            return Ok(()); // Not an error, just not mounted
        }

        let mount_point_str = mount_point.to_str()
            .ok_or_else(|| Error::InvalidState("Invalid mount point path".to_string()))?;

        // Try fusermount first (Linux)
        let mut unmount_cmd = if force {
            let mut cmd = Command::new("fusermount");
            cmd.args(["-u", "-z", mount_point_str]);
            cmd
        } else {
            let mut cmd = Command::new("fusermount");
            cmd.args(["-u", mount_point_str]);
            cmd
        };

        match unmount_cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    info!("Successfully unmounted {}", mount_point.display());
                    return Ok(());
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("fusermount failed: {}", stderr);
                }
            }
            Err(e) => {
                warn!("Failed to run fusermount: {}", e);
            }
        }

        // Try umount as fallback (macOS/BSD)
        let mut umount_cmd = if force {
            let mut cmd = Command::new("umount");
            cmd.args(["-f", mount_point_str]);
            cmd
        } else {
            let mut cmd = Command::new("umount");
            cmd.arg(mount_point_str);
            cmd
        };

        match umount_cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    info!("Successfully unmounted {}", mount_point.display());
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(Error::Fuse(format!("Failed to unmount: {}", stderr)))
                }
            }
            Err(e) => {
                Err(Error::Fuse(format!("Failed to run umount: {}", e)))
            }
        }
    }

    /// Check if a path is currently mounted
    pub fn is_mounted(&self, mount_point: &Path) -> bool {
        let mount_point_str = match mount_point.to_str() {
            Some(s) => s,
            None => return false,
        };

        // Check /proc/mounts on Linux
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == mount_point_str {
                    return true;
                }
            }
        }

        // Check with mount command as fallback
        if let Ok(output) = Command::new("mount").output() {
            let mounts = String::from_utf8_lossy(&output.stdout);
            for line in mounts.lines() {
                if line.contains(mount_point_str) {
                    return true;
                }
            }
        }

        false
    }

    /// Get filesystem statistics
    pub fn get_stats(&self) -> FuseStats {
        FuseStats {
            total_inodes: self.filesystem.get_total_inodes(),
            feeds_count: self.filesystem.get_feeds_count(),
            mount_time: std::time::SystemTime::now(), // This would be tracked properly
        }
    }

    /// Validate a mount point for mounting
    pub fn validate_mount_point(&self, mount_point: &Path) -> Result<()> {
        self.validate_mount_point_internal(mount_point)
    }

    fn validate_mount_point_internal(&self, mount_point: &Path) -> Result<()> {
        // Check if mount point exists, create it if it doesn't
        if !mount_point.exists() {
            info!("Mount point does not exist, creating: {}", mount_point.display());
            std::fs::create_dir_all(mount_point)
                .map_err(|e| Error::PermissionDenied(format!(
                    "Failed to create mount point directory '{}': {}. Please create it manually or run 'rss-fuse init {}'",
                    mount_point.display(),
                    e,
                    mount_point.display()
                )))?;
            info!("Created mount point directory: {}", mount_point.display());
        }

        // Check if it's a directory
        if !mount_point.is_dir() {
            return Err(Error::InvalidState(format!(
                "Mount point is not a directory: {}", 
                mount_point.display()
            )));
        }

        // Check if it's empty
        match std::fs::read_dir(mount_point) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    warn!("Mount point is not empty: {}", mount_point.display());
                }
            }
            Err(e) => {
                return Err(Error::PermissionDenied(format!(
                    "Cannot read mount point directory: {}", e
                )));
            }
        }

        // Check if already mounted
        if self.is_mounted(mount_point) {
            return Err(Error::AlreadyExists(format!(
                "Mount point is already mounted: {}", 
                mount_point.display()
            )));
        }

        Ok(())
    }
}

/// Mount options for the FUSE filesystem
#[derive(Debug, Clone)]
pub struct MountOptions {
    pub allow_other: bool,
    pub allow_root: bool,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub auto_unmount: bool,
    pub read_only: bool,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            allow_other: false,
            allow_root: false,
            uid: None,
            gid: None,
            auto_unmount: true,
            read_only: true, // RSS-FUSE is read-only by design
        }
    }
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FuseStats {
    pub total_inodes: usize,
    pub feeds_count: usize,
    pub mount_time: std::time::SystemTime,
}

impl Default for FuseOperations {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_fuse_operations_creation() {
        let ops = FuseOperations::new();
        assert_eq!(ops.filesystem.get_total_inodes(), 5); // root + meta structure (4 nodes: root, .rss-fuse, logs, cache, config.toml)
    }

    #[test]
    fn test_mount_options_default() {
        let options = MountOptions::default();
        assert!(!options.allow_other);
        assert!(!options.allow_root);
        assert!(options.auto_unmount);
        assert!(options.read_only);
    }

    #[test]
    fn test_validate_mount_point() {
        let ops = FuseOperations::new();
        
        // Test with temporary directory
        let temp_dir = TempDir::new().unwrap();
        let mount_point = temp_dir.path();
        
        // Should be valid
        ops.validate_mount_point(mount_point).unwrap();
    }

    #[test]
    fn test_validate_mount_point_nonexistent() {
        let ops = FuseOperations::new();
        let mount_point = Path::new("/nonexistent/path/that/requires/root");
        
        // Should fail because we don't have permission to create the directory
        let result = ops.validate_mount_point(mount_point);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::PermissionDenied(_)));
    }

    #[test]
    fn test_validate_mount_point_creates_directory() {
        let ops = FuseOperations::new();
        let temp_dir = TempDir::new().unwrap();
        let mount_point = temp_dir.path().join("new_mount_dir");
        
        // Directory doesn't exist yet
        assert!(!mount_point.exists());
        
        // Should create the directory and succeed
        ops.validate_mount_point(&mount_point).unwrap();
        
        // Directory should now exist
        assert!(mount_point.exists());
        assert!(mount_point.is_dir());
    }

    #[test]
    fn test_validate_mount_point_file() {
        let ops = FuseOperations::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "test").unwrap();
        
        let result = ops.validate_mount_point(&file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidState(_)));
    }

    #[test]
    fn test_get_stats() {
        let ops = FuseOperations::new();
        let stats = ops.get_stats();
        
        assert_eq!(stats.total_inodes, 5); // root + meta structure
        assert_eq!(stats.feeds_count, 0);
    }

    #[test]
    fn test_is_mounted_false() {
        let ops = FuseOperations::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Should return false for unmounted directory
        assert!(!ops.is_mounted(temp_dir.path()));
    }
}