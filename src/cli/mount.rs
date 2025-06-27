use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::io::Write;
use tokio::signal;
use tracing::{info, warn, error, debug};

use crate::config::Config;
use crate::storage::{Repository, RepositoryFactory, CacheConfig, PersistentCacheConfig, FeedRepository};
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
    let mount_start = std::time::Instant::now();
    
    // Load configuration
    print!("⚡ Initializing RSS-FUSE... ");
    std::io::stdout().flush().unwrap();
    let config_file = get_config_file(config_path)?;
    let config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound(
            "Configuration file not found. Run 'rss-fuse init' first.".to_string()
        ));
    };
    println!("✅ ({:.0}ms)", mount_start.elapsed().as_millis());
    
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
        max_memory_mb: config.cache.max_size_mb as usize,
    };

    // Setup persistent cache configuration
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/tmp".into()))
        .join("rss-fuse");
    
    let persistent_config = PersistentCacheConfig {
        cache_dir,
        max_age_days: 7, // Keep cache for 1 week
        max_size_mb: config.cache.max_size_mb as u64,
        enable_compression: true,
    };
    
    let repo = Arc::new(RepositoryFactory::with_persistent_cache(
        crate::storage::StorageConfig::default(),
        cache_config,
        persistent_config,
    ).map_err(|e| Error::Storage(format!("Failed to create repository with persistent cache: {}", e)))?);
    
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
    print!("🔍 Validating mount point... ");
    std::io::stdout().flush().unwrap();
    match fuse_ops.validate_mount_point(&mount_point) {
        Ok(_) => {
            println!("✅ ({:.0}ms)", mount_start.elapsed().as_millis());
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
    
    // Start cache-first loading task
    let cache_repo = repo.clone();
    let cache_config = config.clone();
    let cache_fuse = Arc::clone(&fuse_ops.filesystem);
    
    tokio::spawn(async move {
        info!("Starting cache-first feed loading");
        
        // Phase 1: Load cached content immediately
        for (name, url) in &cache_config.feeds {
            debug!("Checking cache for feed: {}", name);
            
            match cache_repo.load_feed_cache_first(name, url).await {
                Ok(Some(feed)) => {
                    info!("Found cached feed: {} ({} articles, age: {:?})", 
                          name, feed.articles.len(), 
                          feed.last_updated.map(|t| chrono::Utc::now().signed_duration_since(t)));
                    
                    // Add cached content immediately
                    if let Err(e) = cache_fuse.add_feed_from_cache(feed, true) {
                        error!("Failed to add cached feed {} to filesystem: {}", name, e);
                    }
                },
                Ok(None) => {
                    debug!("No cached content for feed: {}", name);
                    // Keep loading placeholder - background refresh will update it
                },
                Err(e) => {
                    warn!("Failed to load cached feed {}: {}", name, e);
                }
            }
        }
        
        info!("Cache loading phase completed");
    });
    
    // Start background refresh task (runs immediately for fresh content)
    let refresh_repo = repo.clone();
    let refresh_config = config.clone();
    let refresh_fuse = Arc::clone(&fuse_ops.filesystem);
    
    tokio::spawn(async move {
        info!("Starting background feed refresh");
        
        // Small delay to let cache loading complete first
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        for (name, url) in &refresh_config.feeds {
            debug!("Background refreshing feed: {} from {}", name, url);
            
            match refresh_repo.refresh_feed_background(name, url).await {
                Ok(Some(feed)) => {
                    info!("Successfully refreshed feed: {} ({} articles)", name, feed.articles.len());
                    
                    // Update filesystem with fresh content
                    if let Err(e) = refresh_fuse.add_feed_from_cache(feed, false) {
                        error!("Failed to update refreshed feed {} in filesystem: {}", name, e);
                    }
                },
                Ok(None) => {
                    debug!("Background refresh failed for feed: {} (cached content remains)", name);
                },
                Err(e) => {
                    error!("Background refresh error for feed {}: {}", name, e);
                    
                    // Only add error placeholder if we don't have cached content
                    if refresh_repo.get_feed(name).await.unwrap_or(None).is_none() {
                        if let Err(err) = refresh_fuse.add_error_placeholder(name, &e.to_string()) {
                            error!("Failed to add error placeholder for {}: {}", name, err);
                        }
                    }
                }
            }
        }
        
        info!("Background refresh completed");
    });
    
    // Start periodic refresh task  
    let periodic_repo = repo.clone();
    let periodic_config = config.clone();
    let periodic_fuse = Arc::clone(&fuse_ops.filesystem);
    
    tokio::spawn(async move {
        // Wait for initial loading and background refresh to complete
        tokio::time::sleep(Duration::from_secs(30)).await;
        
        let interval_secs = periodic_config.settings.refresh_interval;
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        
        loop {
            interval.tick().await;
            info!("Running periodic feed refresh (interval: {}s)", interval_secs);
            
            // Create a vector of tasks for parallel refresh
            let mut refresh_tasks = Vec::new();
            
            for (name, url) in &periodic_config.feeds {
                let repo = periodic_repo.clone();
                let fuse = Arc::clone(&periodic_fuse);
                let feed_name = name.clone();
                let feed_url = url.clone();
                
                let task = tokio::spawn(async move {
                    match repo.refresh_feed_background(&feed_name, &feed_url).await {
                        Ok(Some(feed)) => {
                            debug!("Periodic refresh: {} ({} articles)", feed_name, feed.articles.len());
                            
                            // Update FUSE filesystem with fresh content
                            if let Err(e) = fuse.add_feed_from_cache(feed, false) {
                                warn!("Failed to update refreshed feed {} in filesystem: {}", feed_name, e);
                            }
                        },
                        Ok(None) => {
                            debug!("Periodic refresh failed for {}, keeping cached content", feed_name);
                        },
                        Err(e) => {
                            warn!("Periodic refresh error for {}: {}", feed_name, e);
                        }
                    }
                });
                
                refresh_tasks.push(task);
            }
            
            // Wait for all refresh tasks to complete
            for task in refresh_tasks {
                let _ = task.await;
            }
            
            debug!("Periodic refresh cycle completed");
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
    let result = if foreground {
        mount_foreground(fuse_ops, mount_point.clone(), mount_options, file_manager_launcher, repo.clone()).await
    } else if daemon {
        mount_daemon(fuse_ops, mount_point.clone(), mount_options, file_manager_launcher, repo.clone()).await
    } else {
        // Default to foreground mode for now
        mount_foreground(fuse_ops, mount_point.clone(), mount_options, file_manager_launcher, repo.clone()).await
    };

    if result.is_ok() {
        println!("⚡ Total startup time: {:.0}ms", mount_start.elapsed().as_millis());
    }

    result
}

/// Mount filesystem in foreground mode
async fn mount_foreground(
    fuse_ops: FuseOperations,
    mount_point: PathBuf,
    mount_options: MountOptions,
    file_manager_launcher: FileManagerLauncher,
    repo: Arc<Repository>,
) -> Result<()> {
    println!("\n🚀 Starting RSS-FUSE filesystem...");
    println!("   Mode: Foreground");
    println!("   Mount point: {}", mount_point.display());
    println!("   Press Ctrl+C to unmount and exit");
    println!("");
    
    print!("🔗 Mounting filesystem... ");
    std::io::stdout().flush().unwrap();
    let mount_time = std::time::Instant::now();
    info!("Mounting filesystem at {}", mount_point.display());
    
    // For now, we'll simulate the mount and wait for signal
    match fuse_ops.mount(&mount_point, mount_options) {
        Ok(_) => {
            println!("✅ ({:.0}ms)", mount_time.elapsed().as_millis());
            println!("📂 Filesystem ready at: {}", mount_point.display());
            println!("   RSS feeds are loading in the background...");
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
            
            // Save cache before unmounting
            println!("💾 Saving cache to disk...");
            if let Err(e) = repo.save_cache() {
                warn!("Failed to save cache on shutdown: {}", e);
            } else {
                println!("✅ Cache saved successfully");
            }
            
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
    _repo: Arc<Repository>,
) -> Result<()> {
    println!("\n🚀 Starting RSS-FUSE daemon...");
    println!("   Mode: Background (daemon)");
    println!("   Mount point: {}", mount_point.display());
    
    print!("🔗 Mounting filesystem... ");
    std::io::stdout().flush().unwrap();
    let mount_time = std::time::Instant::now();
    // In a real implementation, this would fork and daemonize
    // For now, we'll just mount and detach
    match fuse_ops.mount(&mount_point, mount_options) {
        Ok(_) => {
            println!("✅ ({:.0}ms)", mount_time.elapsed().as_millis());
            println!("📂 Daemon started successfully!");
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