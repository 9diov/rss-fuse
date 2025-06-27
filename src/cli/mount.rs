use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{info, warn, error, debug};

use crate::config::Config;
use crate::storage::{Repository, RepositoryFactory, CacheConfig, FeedRepository};
use crate::fuse::{FuseOperations, MountOptions};
use crate::file_manager::FileManagerLauncher;
use crate::error::{Error, Result};

/// Mount RSS feeds as a FUSE filesystem
pub async fn mount(
    mount_point: PathBuf,
    daemon: bool,
    allow_other: bool,
    foreground: bool,
    no_auto_open: bool,
    file_manager_override: Option<String>,
    config_path: Option<PathBuf>,
) -> Result<()> {
    info!("Mounting RSS-FUSE at: {}", mount_point.display());
    
    // Load configuration
    let config_file = get_config_file(config_path)?;
    let config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound(
            "Configuration file not found. Run 'rss-fuse init' first.".to_string()
        ));
    };
    
    if config.feeds.is_empty() {
        warn!("No feeds configured. The filesystem will be empty.");
        println!("⚠️  No feeds configured yet.");
        println!("   Add feeds with: rss-fuse add-feed <name> <url>");
        println!("   The filesystem will be mounted but empty.");
        println!("");
    }
    
    // Create repository with cache configuration
    let cache_config = CacheConfig {
        max_entries: 1000,
        default_ttl: Duration::from_secs(config.settings.cache_duration),
        cleanup_interval: Duration::from_secs(300),
        max_memory_mb: 100,
    };
    
    let repo = RepositoryFactory::with_config(
        crate::storage::StorageConfig::default(),
        cache_config,
    );
    
    // Create FUSE operations first
    let fuse_ops = FuseOperations::new();
    
    // Check if mount point is already mounted
    if fuse_ops.is_mounted(&mount_point) {
        println!("⚠️  Mount point is already mounted: {}", mount_point.display());
        println!("   Current mount appears to be active.");
        println!("");
        
        // Offer options to the user
        println!("   Options:");
        println!("   1. Unmount first: rss-fuse unmount {}", mount_point.display());
        println!("   2. Use a different mount point");
        println!("   3. Force unmount and remount: rss-fuse unmount --force {} && rss-fuse mount {}", 
                 mount_point.display(), mount_point.display());
        
        return Err(Error::AlreadyExists(format!(
            "Mount point {} is already mounted. Unmount it first or use a different path.",
            mount_point.display()
        )));
    }
    
    // Check mount point validity and handle stale mounts
    match fuse_ops.validate_mount_point(&mount_point) {
        Ok(_) => {
            info!("Mount point validation passed: {}", mount_point.display());
        },
        Err(Error::AlreadyExists(_)) => {
            // This shouldn't happen since we checked above, but handle anyway
            return Err(Error::AlreadyExists(format!(
                "Mount point {} is already in use", mount_point.display()
            )));
        },
        Err(e) => {
            // Check if this might be a stale mount
            if mount_point.exists() && fuse_ops.is_mount_stale(&mount_point) {
                println!("🔧 Detected stale mount point: {}", mount_point.display());
                println!("   This appears to be a leftover from a previous session.");
                println!("   Attempting automatic cleanup...");
                
                match fuse_ops.cleanup_stale_mount(&mount_point) {
                    Ok(_) => {
                        println!("✅ Stale mount cleaned up successfully");
                        // Re-validate after cleanup
                        fuse_ops.validate_mount_point(&mount_point)?;
                    },
                    Err(cleanup_err) => {
                        println!("❌ Failed to cleanup stale mount: {}", cleanup_err);
                        println!("   Manual cleanup required:");
                        println!("   fusermount -u {}", mount_point.display());
                        println!("   # or");
                        println!("   rss-fuse unmount --force {}", mount_point.display());
                        return Err(e);
                    }
                }
            } else {
                return Err(e);
            }
        }
    }
    
    // Create placeholder directories for all configured feeds
    println!("📂 Setting up feed placeholders...");
    for (name, _url) in &config.feeds {
        if let Err(e) = fuse_ops.filesystem.add_loading_placeholder(name) {
            warn!("Failed to create placeholder for {}: {}", name, e);
        } else {
            println!("   📁 {} (loading...)", name);
        }
    }
    
    if !config.feeds.is_empty() {
        println!("✅ Created {} feed placeholders", config.feeds.len());
        println!("   Feeds will load in the background after mounting");
    } else {
        warn!("No feeds configured. The filesystem will be empty.");
        println!("⚠️  No feeds configured yet.");
        println!("   Add feeds with: rss-fuse add-feed <name> <url>");
        println!("   The filesystem will be mounted but empty.");
        println!("");
    }
    
    // Configure mount options - disable auto_unmount to avoid permission issues
    let mount_options = MountOptions {
        allow_other: false, // Disable to avoid requiring /etc/fuse.conf changes
        allow_root: false,
        uid: None,
        gid: None,
        auto_unmount: false, // Disable to avoid auto-enabling allow_other
        read_only: true,
    };
    
    // Mount point has already been validated above
    
    println!("\n🎯 Mount Summary:");
    println!("   📂 Feeds configured: {}", config.feeds.len());
    println!("   📁 Mount point: {}", mount_point.display());
    println!("   🔧 Options: {}", format_mount_options(&mount_options));
    
    // Start initial feed loading task (runs once immediately after mount)
    let initial_repo = repo.clone();
    let initial_config = config.clone();
    let initial_fuse = Arc::clone(&fuse_ops.filesystem);
    
    tokio::spawn(async move {
        info!("Starting initial feed loading in background");
        
        for (name, url) in &initial_config.feeds {
            debug!("Loading feed: {} from {}", name, url);
            
            match initial_repo.refresh_feed(name, url).await {
                Ok(feed) => {
                    info!("Successfully loaded feed: {} ({} articles)", name, feed.articles.len());
                    
                    // Replace placeholder with actual feed content
                    if let Err(e) = initial_fuse.add_feed(feed) {
                        error!("Failed to add loaded feed {} to filesystem: {}", name, e);
                    }
                },
                Err(e) => {
                    error!("Failed to load feed {}: {}", name, e);
                    
                    // Update placeholder with error information
                    if let Err(err) = initial_fuse.add_error_placeholder(name, &e.to_string()) {
                        error!("Failed to add error placeholder for {}: {}", name, err);
                    }
                }
            }
        }
        
        info!("Initial feed loading completed");
    });
    
    // Start periodic refresh task
    let refresh_repo = repo.clone();
    let refresh_config = config.clone();
    let refresh_fuse = Arc::clone(&fuse_ops.filesystem);
    
    tokio::spawn(async move {
        // Wait for initial loading to complete before starting periodic refresh
        tokio::time::sleep(Duration::from_secs(30)).await;
        
        let interval_secs = refresh_config.settings.refresh_interval;
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        
        loop {
            interval.tick().await;
            debug!("Running periodic feed refresh");
            
            for (name, url) in &refresh_config.feeds {
                match refresh_repo.refresh_feed(name, url).await {
                    Ok(feed) => {
                        debug!("Refreshed feed: {} ({} articles)", name, feed.articles.len());
                        
                        // Update FUSE filesystem
                        if let Err(e) = refresh_fuse.add_feed(feed) {
                            warn!("Failed to update feed {} in filesystem: {}", name, e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to refresh feed {}: {}", name, e);
                    }
                }
            }
        }
    });
    
    // Prepare file manager launcher
    let mut file_manager_config = config.fuse.auto_open.clone();
    
    // Apply CLI overrides
    if no_auto_open {
        file_manager_config.enabled = false;
    }
    if let Some(fm_command) = file_manager_override {
        file_manager_config.command = fm_command;
        file_manager_config.auto_detect = false;
    }
    
    let file_manager_launcher = FileManagerLauncher::new(file_manager_config);

    // Mount the filesystem
    if foreground {
        mount_foreground(fuse_ops, mount_point, mount_options, file_manager_launcher).await
    } else if daemon {
        mount_daemon(fuse_ops, mount_point, mount_options, file_manager_launcher).await
    } else {
        // Default to foreground mode for now
        mount_foreground(fuse_ops, mount_point, mount_options, file_manager_launcher).await
    }
}

/// Mount filesystem in foreground mode
async fn mount_foreground(
    fuse_ops: FuseOperations,
    mount_point: PathBuf,
    mount_options: MountOptions,
    file_manager_launcher: FileManagerLauncher,
) -> Result<()> {
    println!("\n🚀 Starting RSS-FUSE filesystem...");
    println!("   Mode: Foreground");
    println!("   Mount point: {}", mount_point.display());
    println!("   Press Ctrl+C to unmount and exit");
    println!("");
    
    // Simulate mounting (in a real implementation, this would use fuser::mount)
    info!("Mounting filesystem at {}", mount_point.display());
    
    // For now, we'll simulate the mount and wait for signal
    match fuse_ops.mount(&mount_point, mount_options) {
        Ok(_) => {
            println!("✅ Filesystem mounted successfully!");
            println!("   You can now access your RSS feeds at: {}", mount_point.display());
            println!("   Use 'ls {}' to see your feeds", mount_point.display());
            println!("");
            
            // Launch file manager if configured
            if let Err(e) = file_manager_launcher.launch(&mount_point).await {
                warn!("Failed to launch file manager: {}", e);
                println!("⚠️  File manager auto-launch failed: {}", e);
                println!("   You can manually open: {}", mount_point.display());
            } else if file_manager_launcher.config.enabled {
                println!("🎯 File manager launched at: {}", mount_point.display());
            }
            
            // Wait for shutdown signal
            wait_for_shutdown().await;
            
            println!("\n🔄 Shutting down...");
            
            // Unmount filesystem
            if let Err(e) = fuse_ops.unmount(&mount_point, false) {
                warn!("Failed to unmount filesystem: {}", e);
            } else {
                println!("✅ Filesystem unmounted successfully");
            }
        },
        Err(e) => {
            error!("Failed to mount filesystem: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Mount filesystem in daemon mode
async fn mount_daemon(
    fuse_ops: FuseOperations,
    mount_point: PathBuf,
    mount_options: MountOptions,
    file_manager_launcher: FileManagerLauncher,
) -> Result<()> {
    println!("\n🚀 Starting RSS-FUSE daemon...");
    println!("   Mode: Background (daemon)");
    println!("   Mount point: {}", mount_point.display());
    
    // In a real implementation, this would fork and daemonize
    // For now, we'll just mount and detach
    match fuse_ops.mount(&mount_point, mount_options) {
        Ok(_) => {
            println!("✅ Daemon started successfully!");
            println!("   Filesystem mounted at: {}", mount_point.display());
            println!("   Use 'rss-fuse unmount {}' to stop", mount_point.display());
            
            // Launch file manager if configured (in daemon mode, launch and detach)
            if let Err(e) = file_manager_launcher.launch(&mount_point).await {
                warn!("Failed to launch file manager: {}", e);
                println!("⚠️  File manager auto-launch failed: {}", e);
            } else if file_manager_launcher.config.enabled {
                println!("🎯 File manager launched at: {}", mount_point.display());
            }
            
            // In daemon mode, we would typically detach from the terminal
            // For this demo, we'll just return success
        },
        Err(e) => {
            error!("Failed to start daemon: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Unmount the RSS-FUSE filesystem
pub async fn unmount(mount_point: PathBuf, force: bool) -> Result<()> {
    info!("Unmounting RSS-FUSE from: {}", mount_point.display());
    
    println!("🔄 Unmounting RSS-FUSE...");
    println!("   Mount point: {}", mount_point.display());
    if force {
        println!("   Mode: Force unmount");
    }
    
    // Check if mount point exists first
    if !mount_point.exists() {
        println!("⚠️  Mount point does not exist: {}", mount_point.display());
        if !force {
            println!("   This usually means:");
            println!("   • The filesystem was never mounted");
            println!("   • The mount point directory was deleted");
            println!("   • The filesystem was already unmounted");
            println!();
            println!("   Try 'rss-fuse init {}' to recreate the mount point", mount_point.display());
            return Ok(());
        } else {
            println!("   Continuing with force flag to attempt cleanup...");
        }
    }
    
    let fuse_ops = FuseOperations::new();
    
    match fuse_ops.unmount(&mount_point, force) {
        Ok(_) => {
            println!("✅ Filesystem unmounted successfully!");
        },
        Err(e) => {
            if force {
                warn!("Force unmount completed with warnings: {}", e);
                println!("⚠️  Force unmount completed with warnings");
                println!("   The mount point may still need manual cleanup if issues persist.");
            } else {
                error!("Failed to unmount filesystem: {}", e);
                println!("❌ Failed to unmount filesystem");
                
                // Check if it's a busy mount point error
                let error_msg = e.to_string();
                if error_msg.contains("busy") || error_msg.contains("Device or resource busy") {
                    println!("   📋 Mount point is busy - here's how to fix it:");
                    println!("   ");
                    println!("   1. Close any terminals or file managers in the mount directory");
                    println!("   2. Check what's using the mount:");
                    println!("      lsof +D {}", mount_point.display());
                    println!("   ");
                    println!("   3. Force unmount:");
                    println!("      rss-fuse unmount --force {}", mount_point.display());
                    println!("   ");
                    println!("   4. If still stuck, manual cleanup:");
                    println!("      fusermount -u -z {}", mount_point.display());
                    println!("      # or");
                    println!("      sudo umount -l {}", mount_point.display());
                } else {
                    // Provide helpful suggestions based on error type
                    match &e {
                        Error::NotFound(_) => {
                            println!("   Suggestions:");
                            println!("   • Try 'rss-fuse init {}' to recreate the mount point", mount_point.display());
                            println!("   • Use --force flag if you need to clean up");
                        },
                        _ => {
                            println!("   Try using --force flag if the filesystem is stuck");
                        }
                    }
                }
                return Err(e);
            }
        }
    }
    
    Ok(())
}

/// Wait for shutdown signal (Ctrl+C)
async fn wait_for_shutdown() {
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        },
        Err(err) => {
            warn!("Failed to listen for shutdown signal: {}", err);
        },
    }
}

/// Format mount options for display
fn format_mount_options(options: &MountOptions) -> String {
    let mut opts = Vec::new();
    
    if options.allow_other {
        opts.push("allow_other");
    }
    if options.allow_root {
        opts.push("allow_root");
    }
    if options.auto_unmount {
        opts.push("auto_unmount");
    }
    if options.read_only {
        opts.push("read_only");
    }
    
    if opts.is_empty() {
        "default".to_string()
    } else {
        opts.join(", ")
    }
}

/// Get the configuration file path
fn get_config_file(config_path: Option<PathBuf>) -> Result<PathBuf> {
    match config_path {
        Some(path) => Ok(path),
        None => {
            let config_dir = get_config_dir()?;
            Ok(config_dir.join("config.toml"))
        },
    }
}

/// Get the configuration directory path
fn get_config_dir() -> Result<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        Ok(PathBuf::from(config_home).join("rss-fuse"))
    } else if let Some(home) = std::env::var_os("HOME") {
        Ok(PathBuf::from(home).join(".config").join("rss-fuse"))
    } else {
        Err(Error::Config("Cannot determine configuration directory".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_format_mount_options() {
        let options = MountOptions {
            allow_other: true,
            allow_root: false,
            uid: None,
            gid: None,
            auto_unmount: true,
            read_only: true,
        };
        
        let formatted = format_mount_options(&options);
        assert!(formatted.contains("allow_other"));
        assert!(formatted.contains("auto_unmount"));
        assert!(formatted.contains("read_only"));
        assert!(!formatted.contains("allow_root"));
    }
    
    #[test]
    fn test_format_mount_options_default() {
        let options = MountOptions {
            allow_other: false,
            allow_root: false,
            uid: None,
            gid: None,
            auto_unmount: false,
            read_only: false,
        };
        
        let formatted = format_mount_options(&options);
        assert_eq!(formatted, "default");
    }
    
    #[tokio::test]
    async fn test_unmount_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let mount_point = temp_dir.path().join("nonexistent");
        
        // Should handle non-existent mount points gracefully
        let result = unmount(mount_point, false).await;
        // We expect this to fail, but it shouldn't panic
        assert!(result.is_err());
    }
}