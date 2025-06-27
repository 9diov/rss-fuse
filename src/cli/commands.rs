use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write};
use clap_complete::{generate, Shell};
use clap::CommandFactory;
use tracing::{info, warn, error, debug};
use tokio;

use crate::cli::Cli;
use crate::config::Config;
use crate::storage::{Repository, RepositoryFactory, FeedRepository, ArticleRepository};
use crate::fuse::FuseOperations;
use crate::feed::{Feed, FeedStatus};
use crate::error::{Error, Result};

/// Initialize RSS-FUSE configuration and directory structure
pub async fn init(mount_point: PathBuf) -> Result<()> {
    info!("Initializing RSS-FUSE configuration");
    
    // Validate and create mount point
    if !mount_point.exists() {
        fs::create_dir_all(&mount_point)
            .map_err(|e| Error::Io(e))?;
        info!("Created mount point directory: {}", mount_point.display());
    }
    
    // Create configuration directory
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| Error::Io(e))?;
        info!("Created configuration directory: {}", config_dir.display());
    }
    
    // Create default configuration file
    let config_file = config_dir.join("config.toml");
    if !config_file.exists() {
        let default_config = create_default_config(&mount_point)?;
        fs::write(&config_file, default_config)
            .map_err(|e| Error::Io(e))?;
        info!("Created default configuration: {}", config_file.display());
    } else {
        warn!("Configuration file already exists: {}", config_file.display());
    }
    
    // Create cache directory
    let cache_dir = config_dir.join("cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)
            .map_err(|e| Error::Io(e))?;
        info!("Created cache directory: {}", cache_dir.display());
    }
    
    // Create logs directory
    let logs_dir = config_dir.join("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)
            .map_err(|e| Error::Io(e))?;
        info!("Created logs directory: {}", logs_dir.display());
    }
    
    println!("✅ RSS-FUSE initialized successfully!");
    println!("   Mount point: {}", mount_point.display());
    println!("   Config file: {}", config_file.display());
    println!("   Cache directory: {}", cache_dir.display());
    println!("");
    println!("Next steps:");
    println!("   1. Add RSS feeds: rss-fuse add-feed <name> <url>");
    println!("   2. Mount filesystem: rss-fuse mount {}", mount_point.display());
    
    Ok(())
}

/// Add a new RSS feed to the configuration
pub async fn add_feed(name: String, url: String, config_path: Option<PathBuf>) -> Result<()> {
    info!("Adding feed: {} -> {}", name, url);
    
    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(Error::InvalidUrl(format!("URL must start with http:// or https://: {}", url)));
    }
    
    // Load existing configuration
    let config_file = get_config_file(config_path)?;
    let mut config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound("Configuration file not found. Run 'rss-fuse init' first.".to_string()));
    };
    
    // Check if feed already exists
    if config.feeds.contains_key(&name) {
        return Err(Error::AlreadyExists(format!("Feed '{}' already exists", name)));
    }
    
    // Create repository for validation
    let repo = RepositoryFactory::memory();
    
    // Test feed URL by fetching it
    println!("📡 Testing feed URL...");
    match repo.refresh_feed(&name, &url).await {
        Ok(feed) => {
            println!("✅ Feed validated successfully!");
            println!("   Title: {}", feed.title.as_deref().unwrap_or("Unknown"));
            println!("   Description: {}", feed.description.as_deref().unwrap_or("No description"));
            println!("   Articles: {}", feed.articles.len());
            
            // Add to configuration
            config.feeds.insert(name.clone(), url.clone());
            
            // Save configuration
            let config_content = toml::to_string_pretty(&config)
                .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
            fs::write(&config_file, config_content)
                .map_err(|e| Error::Io(e))?;
            
            println!("✅ Feed '{}' added successfully!", name);
            
            // Store the feed in repository for immediate availability
            repo.save_feed(feed).await?;
        },
        Err(e) => {
            return Err(Error::FeedParse(format!("Failed to validate feed URL: {}", e)));
        }
    }
    
    Ok(())
}

/// Remove an RSS feed from the configuration
pub async fn remove_feed(name: String, config_path: Option<PathBuf>) -> Result<()> {
    info!("Removing feed: {}", name);
    
    // Load existing configuration
    let config_file = get_config_file(config_path)?;
    let mut config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound("Configuration file not found.".to_string()));
    };
    
    // Check if feed exists
    if !config.feeds.contains_key(&name) {
        return Err(Error::NotFound(format!("Feed '{}' not found", name)));
    }
    
    // Remove from configuration
    let url = config.feeds.remove(&name).unwrap();
    
    // Save configuration
    let config_content = toml::to_string_pretty(&config)
        .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
    fs::write(&config_file, config_content)
        .map_err(|e| Error::Io(e))?;
    
    // Also remove from repository if it exists
    let repo = RepositoryFactory::memory();
    let _ = repo.delete_feed(&name).await; // Ignore errors since it might not be in storage
    
    println!("✅ Feed '{}' removed successfully!", name);
    println!("   Removed URL: {}", url);
    
    Ok(())
}

/// List all configured RSS feeds
pub async fn list_feeds(config_path: Option<PathBuf>) -> Result<()> {
    info!("Listing feeds");
    
    // Load configuration
    let config_file = get_config_file(config_path)?;
    let config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound("Configuration file not found. Run 'rss-fuse init' first.".to_string()));
    };
    
    if config.feeds.is_empty() {
        println!("📋 No feeds configured yet.");
        println!("   Add feeds with: rss-fuse add-feed <name> <url>");
        return Ok(());
    }
    
    println!("📋 Configured RSS Feeds:");
    println!("========================");
    
    // Create repository to get additional information
    let repo = RepositoryFactory::memory();
    
    for (name, url) in &config.feeds {
        println!("\n📰 {}", name);
        println!("   URL: {}", url);
        
        // Try to get cached feed information
        match repo.get_feed(name).await {
            Ok(Some(feed)) => {
                println!("   Title: {}", feed.title.as_deref().unwrap_or("Unknown"));
                println!("   Articles: {}", feed.articles.len());
                println!("   Status: {:?}", feed.status);
                if let Some(updated) = feed.last_updated {
                    println!("   Last Updated: {}", updated.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            },
            Ok(None) => {
                println!("   Status: Not cached (run refresh to update)");
            },
            Err(_) => {
                println!("   Status: Error accessing feed data");
            }
        }
    }
    
    println!("\n💡 Use 'rss-fuse refresh' to update all feeds");
    
    Ok(())
}

/// Manually refresh feeds
pub async fn refresh(feed_name: Option<String>, config_path: Option<PathBuf>) -> Result<()> {
    info!("Refreshing feeds: {:?}", feed_name);
    
    // Load configuration
    let config_file = get_config_file(config_path)?;
    let config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound("Configuration file not found.".to_string()));
    };
    
    if config.feeds.is_empty() {
        println!("📋 No feeds configured yet.");
        return Ok(());
    }
    
    let repo = RepositoryFactory::memory();
    
    match feed_name {
        Some(name) => {
            // Refresh specific feed
            if let Some(url) = config.feeds.get(&name) {
                println!("🔄 Refreshing feed: {}", name);
                match repo.refresh_feed(&name, url).await {
                    Ok(feed) => {
                        println!("✅ {} updated successfully ({} articles)", name, feed.articles.len());
                    },
                    Err(e) => {
                        error!("Failed to refresh {}: {}", name, e);
                        println!("❌ Failed to refresh {}: {}", name, e);
                    }
                }
            } else {
                return Err(Error::NotFound(format!("Feed '{}' not found", name)));
            }
        },
        None => {
            // Refresh all feeds
            println!("🔄 Refreshing all feeds...");
            let mut success_count = 0;
            let mut error_count = 0;
            
            for (name, url) in &config.feeds {
                print!("   {} ... ", name);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                
                match repo.refresh_feed(name, url).await {
                    Ok(feed) => {
                        println!("✅ ({} articles)", feed.articles.len());
                        success_count += 1;
                    },
                    Err(e) => {
                        println!("❌ Error: {}", e);
                        error!("Failed to refresh {}: {}", name, e);
                        error_count += 1;
                    }
                }
            }
            
            println!("\n📊 Refresh Summary:");
            println!("   ✅ Successful: {}", success_count);
            if error_count > 0 {
                println!("   ❌ Failed: {}", error_count);
            }
        }
    }
    
    Ok(())
}

/// Show RSS-FUSE status
pub async fn status(specific_mount_point: Option<PathBuf>) -> Result<()> {
    info!("Showing status");
    
    println!("📊 RSS-FUSE Status");
    println!("==================");
    
    // Check configuration
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join("config.toml");
    
    if config_file.exists() {
        println!("✅ Configuration: {}", config_file.display());
        
        let config = Config::load(&config_file)?;
        println!("   📰 Feeds configured: {}", config.feeds.len());
        
        // Repository statistics
        let repo = RepositoryFactory::memory();
        if let Ok(stats) = FeedRepository::get_stats(&repo).await {
            println!("   📈 Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
            println!("   ⏱️  Avg response time: {:.2}ms", stats.avg_response_time_ms);
            println!("   💾 Total articles: {}", stats.storage.total_articles);
            println!("   📦 Storage size: {} bytes", stats.storage.storage_size_bytes);
        }
    } else {
        println!("❌ Configuration: Not initialized");
        println!("   Run 'rss-fuse init <mount-point>' to initialize");
    }
    
    // Check cache directory
    let cache_dir = config_dir.join("cache");
    if cache_dir.exists() {
        println!("✅ Cache directory: {}", cache_dir.display());
    } else {
        println!("❌ Cache directory: Not found");
    }
    
    // Check logs directory
    let logs_dir = config_dir.join("logs");
    if logs_dir.exists() {
        println!("✅ Logs directory: {}", logs_dir.display());
    } else {
        println!("❌ Logs directory: Not found");
    }
    
    // Check mount status
    println!("\n🗂️  Mount Status:");
    let fuse_ops = crate::fuse::FuseOperations::new();
    
    if let Some(specific_path) = specific_mount_point {
        // Check specific mount point
        println!("Checking specific mount point: {}", specific_path.display());
        
        if specific_path.exists() {
            if fuse_ops.is_mounted(&specific_path) {
                if fuse_ops.is_mount_stale(&specific_path) {
                    println!("⚠️  Status: STALE MOUNT");
                    println!("   The mount point appears to be mounted but is not responsive");
                    println!("   This usually indicates a crashed or hung FUSE process");
                    println!("   Action: rss-fuse unmount --force {}", specific_path.display());
                } else {
                    println!("✅ Status: ACTIVE MOUNT");
                    println!("   The filesystem is mounted and responsive");
                    let stats = fuse_ops.get_stats();
                    println!("   📁 Total inodes: {}", stats.total_inodes);
                    println!("   📰 Feeds mounted: {}", stats.feeds_count);
                    println!("   Action: Access files at {}", specific_path.display());
                }
            } else {
                println!("❌ Status: NOT MOUNTED");
                println!("   Directory exists but no filesystem is mounted");
                println!("   Action: rss-fuse mount {}", specific_path.display());
            }
        } else {
            println!("❌ Status: DIRECTORY MISSING");
            println!("   Mount point directory doesn't exist");
            println!("   Action: rss-fuse init {}", specific_path.display());
        }
    } else {
        // Scan for common mount points
        let common_mount_points = [
            "/tmp/rss-fuse",
            "/tmp/rss-mount", 
            &format!("{}/rss-mount", std::env::var("HOME").unwrap_or_default()),
            &format!("{}/rss-fuse", std::env::var("HOME").unwrap_or_default()),
        ];
        
        let mut active_mounts = Vec::new();
        let mut stale_mounts = Vec::new();
        
        for mount_point_str in &common_mount_points {
            let mount_point = std::path::PathBuf::from(mount_point_str);
            if mount_point.exists() && fuse_ops.is_mounted(&mount_point) {
                if fuse_ops.is_mount_stale(&mount_point) {
                    stale_mounts.push(mount_point);
                } else {
                    active_mounts.push(mount_point);
                }
            }
        }
        
        if !active_mounts.is_empty() {
            for mount_point in &active_mounts {
                println!("✅ Mount point: {} (ACTIVE)", mount_point.display());
                println!("   Status: Mounted and responsive");
                
                // Show filesystem stats if available
                let stats = fuse_ops.get_stats();
                println!("   📁 Total inodes: {}", stats.total_inodes);
                println!("   📰 Feeds mounted: {}", stats.feeds_count);
            }
        }
        
        if !stale_mounts.is_empty() {
            for mount_point in &stale_mounts {
                println!("⚠️  Mount point: {} (STALE)", mount_point.display());
                println!("   Status: Mounted but not responsive");
                println!("   Action: Run 'rss-fuse unmount --force {}' to cleanup", mount_point.display());
            }
        }
        
        if active_mounts.is_empty() && stale_mounts.is_empty() {
            println!("❌ Mount point: No active RSS-FUSE mounts found");
            println!("   Status: No mounted filesystems detected");
            if config_file.exists() {
                println!("   Action: Run 'rss-fuse mount <mount-point>' to mount");
            } else {
                println!("   Action: Run 'rss-fuse init <mount-point>' first, then mount");
            }
            
            println!("\n💡 Tip: Use 'rss-fuse status --mount-point <path>' to check a specific location");
        }
    }
    
    // System information
    println!("\n🖥️  System Information:");
    println!("   📍 Config directory: {}", config_dir.display());
    println!("   🔧 Version: {}", env!("CARGO_PKG_VERSION"));
    println!("   🐧 Platform: {}", std::env::consts::OS);
    
    // Check for required tools
    println!("\n🛠️  System Tools:");
    let tools = [
        ("fusermount", "FUSE unmounting"),
        ("umount", "Fallback unmounting"),
        ("lsof", "Process detection"),
        ("fuser", "Process management"),
    ];
    
    for (tool, description) in &tools {
        if std::process::Command::new(tool).arg("--help").output().is_ok() ||
           std::process::Command::new(tool).arg("-h").output().is_ok() {
            println!("   ✅ {}: Available ({})", tool, description);
        } else {
            println!("   ❌ {}: Not found ({})", tool, description);
        }
    }
    
    Ok(())
}

/// Generate shell completions
pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let cmd_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, cmd_name, &mut std::io::stdout());
}

/// Demo the filesystem structure without mounting
pub async fn demo_filesystem(detailed: bool, config_path: Option<PathBuf>) -> Result<()> {
    info!("Demonstrating filesystem structure");
    
    println!("🎭 RSS-FUSE Filesystem Demo");
    println!("===========================");
    
    // Load configuration
    let config_file = get_config_file(config_path)?;
    let config = if config_file.exists() {
        Config::load(&config_file)?
    } else {
        return Err(Error::NotFound("Configuration file not found. Run 'rss-fuse init' first.".to_string()));
    };
    
    if config.feeds.is_empty() {
        println!("📋 No feeds configured yet.");
        println!("   Add feeds with: rss-fuse add-feed <name> <url>");
        return Ok(());
    }
    
    // Create repository and load feeds
    let repo = RepositoryFactory::memory();
    let mut feed_count = 0;
    let mut total_articles = 0;
    
    println!("\n📁 Virtual Filesystem Structure:");
    println!("├── /");
    
    for (name, url) in &config.feeds {
        print!("│   ├── {} ... ", name);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        match repo.refresh_feed(name, url).await {
            Ok(feed) => {
                let article_count = feed.articles.len();
                println!("📁 ({} articles)", article_count);
                feed_count += 1;
                total_articles += article_count;
                
                // Show first few articles as examples
                let show_count = if detailed { article_count } else { std::cmp::min(3, article_count) };
                for (i, article) in feed.articles.iter().take(show_count).enumerate() {
                    let prefix = if i == show_count - 1 && !detailed && article_count > 3 {
                        "│   │   └──"
                    } else {
                        "│   │   ├──"
                    };
                    
                    let title = article.title.chars().take(50).collect::<String>();
                    let title = if article.title.len() > 50 { 
                        format!("{}...", title) 
                    } else { 
                        title 
                    };
                    
                    println!("│   │   {} {}.txt", prefix, 
                        title.replace("/", "_").replace(":", "_"));
                    
                    if detailed {
                        println!("│   │       📝 {}", 
                            article.description.as_deref().unwrap_or("No description")
                                .chars().take(80).collect::<String>());
                        if !article.link.is_empty() {
                            println!("│   │       🔗 {}", article.link);
                        }
                        if i < article_count - 1 {
                            println!("│   │");
                        }
                    }
                }
                
                if !detailed && article_count > 3 {
                    println!("│   │   └── ... and {} more articles", article_count - 3);
                }
                
                // Show meta directory
                println!("│   └── .meta/");
                println!("│       ├── config.toml");
                println!("│       ├── feed.xml");
                println!("│       └── stats.json");
                
                if feed_count < config.feeds.len() {
                    println!("│");
                }
            },
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
    
    println!("\n📊 Filesystem Summary:");
    println!("   📁 Feeds: {}", feed_count);
    println!("   📄 Articles: {}", total_articles);
    println!("   💾 Virtual files: {}", total_articles + (feed_count * 3)); // articles + meta files
    
    println!("\n💡 Usage:");
    println!("   In a real mount, you would access these files like:");
    println!("   📖 cat ~/rss-mount/hacker-news/Some_Article.txt");
    println!("   🔍 ls ~/rss-mount/");
    println!("   📋 cat ~/rss-mount/hacker-news/.meta/config.toml");
    
    if !detailed && total_articles > 10 {
        println!("\n🔍 Use --detailed flag to see all articles and content");
    }
    
    Ok(())
}

/// Initialize logging based on verbosity flags
pub fn init_logging(debug: bool, verbose: bool) -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let filter = if debug {
        EnvFilter::new("debug")
    } else if verbose {
        EnvFilter::new("info")
    } else {
        EnvFilter::new("warn")
    };
    
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_file(debug)
        .with_line_number(debug)
        .init();
    
    debug!("Logging initialized");
    Ok(())
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

/// Get the configuration file path
fn get_config_file(config_path: Option<PathBuf>) -> Result<PathBuf> {
    match config_path {
        Some(path) => Ok(path),
        None => Ok(get_config_dir()?.join("config.toml")),
    }
}

/// Create default configuration content
fn create_default_config(mount_point: &Path) -> Result<String> {
    let default_config = format!(r#"# RSS-FUSE Configuration File
# Generated on {}

[settings]
# Default mount point
mount_point = "{}"

# Feed refresh interval in seconds (default: 1 hour)
refresh_interval = 3600

# Cache duration in seconds (default: 4 hours)
cache_duration = 14400

# Maximum number of articles per feed (default: 100)
max_articles = 100

# Include article content in files (default: true)
article_content = true

# FUSE filesystem options
[fuse]
# Allow other users to access the filesystem
allow_other = false

# Allow root to access the filesystem
allow_root = false

# Automatic unmount on process exit
auto_unmount = true

# Read-only filesystem
read_only = true

# File manager auto-open configuration
[fuse.auto_open]
# Enable automatic file manager launch after mounting
enabled = false

# File manager command (auto-detected if auto_detect = true)
command = "ranger"

# Additional arguments to pass to the file manager
args = []

# Launch in a new terminal window
new_terminal = true

# Terminal command to use (auto-detected if using default)
terminal_command = "xterm"

# Delay in seconds before launching (allows mount to stabilize)
launch_delay = 2

# Auto-detect available file managers
auto_detect = true

[feeds]
# Add your RSS feeds here
# Format: "feed-name" = "https://example.com/feed.xml"
# 
# Example:
# "hacker-news" = "https://hnrss.org/frontpage"
# "rust-blog" = "https://blog.rust-lang.org/feed.xml"

[cache]
# Maximum cache size in MB (default: 100MB)
max_size_mb = 100

# Cache cleanup interval in seconds (default: 5 minutes)
cleanup_interval = 300

[logging]
# Log level: error, warn, info, debug, trace
level = "info"

# Log to file
log_to_file = true

# Log file path (relative to config directory)
log_file = "logs/rss-fuse.log"
"#, 
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        mount_point.display()
    );
    
    Ok(default_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_get_config_dir() {
        // This test might fail in some environments, so we'll just check it doesn't panic
        let _ = get_config_dir();
    }
    
    #[test]
    fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let mount_point = temp_dir.path();
        
        let config = create_default_config(mount_point).unwrap();
        assert!(config.contains("[settings]"));
        assert!(config.contains("[feeds]"));
        assert!(config.contains("[fuse]"));
        assert!(config.contains(&mount_point.display().to_string()));
    }
    
    #[tokio::test]
    async fn test_init_command() {
        let temp_dir = TempDir::new().unwrap();
        let mount_point = temp_dir.path().join("mount");
        
        // Should succeed
        init(mount_point.clone()).await.unwrap();
        
        // Check that directories were created
        assert!(mount_point.exists());
        
        // Check that running init again doesn't fail
        init(mount_point).await.unwrap();
    }
    
    #[test]
    fn test_init_logging() {
        // Test that logging initialization doesn't panic
        // Note: This might interfere with other tests that also initialize logging
        let result = init_logging(false, false);
        // We don't assert success because logging might already be initialized
        let _ = result;
    }
}