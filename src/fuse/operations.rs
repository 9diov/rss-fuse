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
        
        // Give the mount a moment to establish (reduced for faster startup)
        std::thread::sleep(std::time::Duration::from_millis(50));
        
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

        // Strategy 1: Try graceful unmount first with retry
        if !force {
            if let Ok(()) = self.try_graceful_unmount_with_retry(mount_point_str, 3) {
                info!("Successfully unmounted {} gracefully", mount_point.display());
                return Ok(());
            }
        }

        // Strategy 2: Handle busy mount point
        if self.is_mount_busy(mount_point_str) {
            info!("Mount point is busy, attempting recovery strategies...");
            
            // Show what's using the mount point
            self.show_mount_usage(mount_point_str);
            
            // Try to kill processes using the mount point
            if force {
                self.kill_mount_users(mount_point_str)?;
                // Give processes time to exit
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }

        // Strategy 3: Try force unmount
        if let Ok(()) = self.try_force_unmount(mount_point_str, force) {
            info!("Successfully force unmounted {}", mount_point.display());
            return Ok(());
        }

        // Strategy 4: Last resort - lazy unmount
        if force {
            if let Ok(()) = self.try_lazy_unmount(mount_point_str) {
                warn!("Used lazy unmount for {} - mount point will be cleaned up when no longer in use", mount_point.display());
                return Ok(());
            }
        }

        Err(Error::Fuse(format!(
            "Failed to unmount {} - mount point is busy. Try:\n\
            1. Close any file managers or terminals in the mount directory\n\
            2. Run 'lsof +D {}' to see what's using the mount\n\
            3. Use 'rss-fuse unmount --force {}' to force unmount",
            mount_point.display(),
            mount_point_str,
            mount_point_str
        )))
    }

    /// Try graceful unmount with retry mechanism
    fn try_graceful_unmount_with_retry(&self, mount_point_str: &str, max_attempts: u32) -> Result<()> {
        for attempt in 1..=max_attempts {
            debug!("Attempting graceful unmount of {} (attempt {}/{})", mount_point_str, attempt, max_attempts);
            
            if let Ok(()) = self.try_graceful_unmount(mount_point_str) {
                return Ok(());
            }
            
            if attempt < max_attempts {
                info!("Unmount attempt {} failed, retrying in 1 second...", attempt);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        
        Err(Error::Fuse(format!("Failed to unmount {} after {} attempts", mount_point_str, max_attempts)))
    }

    /// Try graceful unmount
    fn try_graceful_unmount(&self, mount_point_str: &str) -> Result<()> {
        debug!("Attempting graceful unmount of {}", mount_point_str);
        
        // Try fusermount first (Linux)
        let output = Command::new("fusermount")
            .args(["-u", mount_point_str])
            .output()
            .map_err(|e| Error::Fuse(format!("Failed to run fusermount: {}", e)))?;

        if output.status.success() {
            return Ok(());
        }

        // Try umount as fallback (macOS/BSD)
        let output = Command::new("umount")
            .arg(mount_point_str)
            .output()
            .map_err(|e| Error::Fuse(format!("Failed to run umount: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(Error::Fuse(format!("Graceful unmount failed: {}", stderr)))
        }
    }

    /// Try force unmount
    fn try_force_unmount(&self, mount_point_str: &str, _force: bool) -> Result<()> {
        debug!("Attempting force unmount of {}", mount_point_str);
        
        // Try fusermount with force flags (Linux)
        let output = Command::new("fusermount")
            .args(["-u", "-z", mount_point_str])
            .output()
            .map_err(|e| Error::Fuse(format!("Failed to run fusermount: {}", e)))?;

        if output.status.success() {
            return Ok(());
        }

        // Try umount with force flag (macOS/BSD)
        let output = Command::new("umount")
            .args(["-f", mount_point_str])
            .output()
            .map_err(|e| Error::Fuse(format!("Failed to run umount: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(Error::Fuse(format!("Force unmount failed: {}", stderr)))
        }
    }

    /// Try lazy unmount (detach immediately, cleanup when no longer in use)
    fn try_lazy_unmount(&self, mount_point_str: &str) -> Result<()> {
        debug!("Attempting lazy unmount of {}", mount_point_str);
        
        // Linux: fusermount with -z (lazy) flag
        let output = Command::new("fusermount")
            .args(["-u", "-z", mount_point_str])
            .output()
            .map_err(|e| Error::Fuse(format!("Failed to run fusermount: {}", e)))?;

        if output.status.success() {
            return Ok(());
        }

        // Linux: umount with -l (lazy) flag
        let output = Command::new("umount")
            .args(["-l", mount_point_str])
            .output()
            .map_err(|e| Error::Fuse(format!("Failed to run umount: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(Error::Fuse(format!("Lazy unmount failed: {}", stderr)))
        }
    }

    /// Check if mount point is busy
    fn is_mount_busy(&self, mount_point_str: &str) -> bool {
        // Check with lsof if available
        if let Ok(output) = Command::new("lsof")
            .args(["+D", mount_point_str])
            .output() {
            !output.stdout.is_empty()
        } else {
            // Fallback: check with fuser if available
            if let Ok(output) = Command::new("fuser")
                .args(["-m", mount_point_str])
                .output() {
                !output.stdout.is_empty()
            } else {
                false
            }
        }
    }

    /// Show what processes are using the mount point
    fn show_mount_usage(&self, mount_point_str: &str) {
        info!("Checking what's using mount point: {}", mount_point_str);
        
        // Try lsof first
        if let Ok(output) = Command::new("lsof")
            .args(["+D", mount_point_str])
            .output() {
            if !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                info!("Processes using mount point:\n{}", stdout);
                return;
            }
        }

        // Try fuser as fallback
        if let Ok(output) = Command::new("fuser")
            .args(["-v", mount_point_str])
            .output() {
            if !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                info!("Processes using mount point:\n{}", stdout);
            }
        }
    }

    /// Kill processes using the mount point (only when force flag is used)
    fn kill_mount_users(&self, mount_point_str: &str) -> Result<()> {
        warn!("Force flag enabled - attempting to kill processes using mount point");
        
        // Try fuser -k (kill processes)
        if let Ok(output) = Command::new("fuser")
            .args(["-k", "-m", mount_point_str])
            .output() {
            if output.status.success() {
                info!("Killed processes using mount point");
                return Ok(());
            }
        }

        // Manual approach with lsof + kill
        if let Ok(output) = Command::new("lsof")
            .args(["-t", "+D", mount_point_str])
            .output() {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    warn!("Killing process {} using mount point", pid);
                    let _ = Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .output();
                }
            }
        }

        Ok(())
    }

    /// Check if a mount point is stale (appears mounted but not responsive)
    pub fn is_mount_stale(&self, mount_point: &Path) -> bool {
        if !self.is_mounted(mount_point) {
            return false;
        }
        
        // Try to access the mount point to see if it's responsive
        match std::fs::read_dir(mount_point) {
            Ok(_) => false, // Mount is responsive
            Err(e) => {
                // Check for common stale mount errors
                let error_msg = e.to_string().to_lowercase();
                error_msg.contains("transport endpoint is not connected") ||
                error_msg.contains("stale file handle") ||
                error_msg.contains("input/output error")
            }
        }
    }
    
    /// Cleanup a stale mount point
    pub fn cleanup_stale_mount(&self, mount_point: &Path) -> Result<()> {
        info!("Attempting to cleanup stale mount: {}", mount_point.display());
        
        let mount_point_str = mount_point.to_str()
            .ok_or_else(|| Error::InvalidState("Invalid mount point path".to_string()))?;
        
        // Try lazy unmount first (safest for stale mounts)
        if let Ok(()) = self.try_lazy_unmount(mount_point_str) {
            info!("Successfully cleaned up stale mount with lazy unmount");
            return Ok(());
        }
        
        // Try force unmount
        if let Ok(()) = self.try_force_unmount(mount_point_str, true) {
            info!("Successfully cleaned up stale mount with force unmount");
            return Ok(());
        }
        
        Err(Error::Fuse(format!("Failed to cleanup stale mount: {}", mount_point.display())))
    }

    /// Check if a path is currently mounted
    pub fn is_mounted(&self, mount_point: &Path) -> bool {
        let mount_point_str = match mount_point.to_str() {
            Some(s) => s,
            None => return false,
        };

        // Check /proc/mounts on Linux (fast system call)
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == mount_point_str {
                    return true;
                }
            }
            // /proc/mounts was readable but mount point not found
            return false;
        }

        // Only use mount command as fallback if /proc/mounts is unavailable
        // (avoid redundant system call for performance)
        if let Ok(output) = Command::new("mount").output() {
            let mounts = String::from_utf8_lossy(&output.stdout);
            return mounts.lines().any(|line| line.contains(mount_point_str));
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