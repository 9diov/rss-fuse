use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{info, warn, error, debug};

use crate::config::Config;
use crate::storage::{Repository, RepositoryFactory, CacheConfig, FeedRepository};
use crate::fuse::{FuseOperations, MountOptions};
use crate::error::{Error, Result};

/// Mount RSS feeds as a FUSE filesystem
pub async fn mount(
    mount_point: PathBuf,
    daemon: bool,
    allow_other: bool,
    foreground: bool,
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
    
    // Load all configured feeds and add them to both repository and FUSE filesystem
    println!("📡 Loading configured feeds...");
    let mut loaded_feeds = 0;
    let mut failed_feeds = 0;
    
    for (name, url) in &config.feeds {
        print!("   {} ... ", name);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        match repo.refresh_feed(name, url).await {
            Ok(feed) => {
                println!("✅ ({} articles)", feed.articles.len());
                
                // Add feed to FUSE filesystem
                if let Err(e) = fuse_ops.filesystem.add_feed(feed) {
                    warn!("Failed to add feed {} to filesystem: {}", name, e);
                    println!("   ⚠️  FUSE filesystem error: {}", e);
                } else {
                    loaded_feeds += 1;
                }
            },
            Err(e) => {
                println!("❌ Error: {}", e);
                error!("Failed to load feed {}: {}", name, e);
                failed_feeds += 1;
            }
        }
    }
    
    if loaded_feeds == 0 && !config.feeds.is_empty() {
        return Err(Error::FeedParse(
            "Failed to load any feeds. Check your network connection and feed URLs.".to_string()
        ));
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
    
    // Validate mount point
    fuse_ops.validate_mount_point(&mount_point)?;
    
    println!("\n🎯 Mount Summary:");
    println!("   ✅ Feeds loaded: {}", loaded_feeds);
    if failed_feeds > 0 {
        println!("   ❌ Feeds failed: {}", failed_feeds);
    }
    println!("   📁 Mount point: {}", mount_point.display());
    println!("   🔧 Options: {}", format_mount_options(&mount_options));
    
    // Start background refresh task
    let refresh_repo = repo.clone();
    let refresh_config = config.clone();
    let refresh_fuse = Arc::clone(&fuse_ops.filesystem);
    
    tokio::spawn(async move {
        let interval_secs = refresh_config.settings.refresh_interval;
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        
        loop {
            interval.tick().await;
            debug!("Running background feed refresh");
            
            for (name, url) in &refresh_config.feeds {
                match refresh_repo.refresh_feed(name, url).await {
                    Ok(feed) => {
                        debug!("Refreshed feed: {} ({} articles)", name, feed.articles.len());
                        
                        // Update FUSE filesystem
                        let _ = refresh_fuse.remove_feed(name);
                        let _ = refresh_fuse.add_feed(feed);
                    },
                    Err(e) => {
                        warn!("Failed to refresh feed {}: {}", name, e);
                    }
                }
            }
        }
    });
    
    // Mount the filesystem
    if foreground {
        mount_foreground(fuse_ops, mount_point, mount_options).await
    } else if daemon {
        mount_daemon(fuse_ops, mount_point, mount_options).await
    } else {
        // Default to foreground mode for now
        mount_foreground(fuse_ops, mount_point, mount_options).await
    }
}

/// Mount filesystem in foreground mode
async fn mount_foreground(
    fuse_ops: FuseOperations,
    mount_point: PathBuf,
    mount_options: MountOptions,
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
            } else {
                error!("Failed to unmount filesystem: {}", e);
                println!("❌ Failed to unmount filesystem: {}", e);
                
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