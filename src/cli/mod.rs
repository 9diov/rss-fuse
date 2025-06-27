pub mod commands;
pub mod mount;

use clap::{Parser, Subcommand};
use crate::error::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rss-fuse")]
#[command(about = "A FUSE filesystem for RSS feeds")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    /// Enable debug output
    #[arg(short, long, global = true)]
    pub debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize RSS-FUSE configuration
    Init {
        /// Mount point directory
        mount_point: PathBuf,
    },
    
    /// Mount RSS feeds as filesystem
    Mount {
        /// Mount point directory
        mount_point: PathBuf,
        
        /// Run in background (daemon mode)
        #[arg(long)]
        daemon: bool,
        
        /// Allow other users to access the filesystem
        #[arg(short, long)]
        allow_other: bool,
        
        /// Foreground mode (do not daemonize)
        #[arg(short, long)]
        foreground: bool,
        
        /// Disable automatic file manager launch
        #[arg(long)]
        no_auto_open: bool,
        
        /// Override file manager command
        #[arg(long)]
        file_manager: Option<String>,
    },
    
    /// Unmount the filesystem
    Unmount {
        /// Mount point directory
        mount_point: PathBuf,
        
        /// Force unmount
        #[arg(short, long)]
        force: bool,
    },
    
    /// Add a new RSS feed
    AddFeed {
        /// Feed name
        name: String,
        
        /// Feed URL
        url: String,
    },
    
    /// Remove an RSS feed
    RemoveFeed {
        /// Feed name
        name: String,
    },
    
    /// List all configured feeds
    ListFeeds,
    
    /// Refresh feeds manually
    Refresh {
        /// Specific feed name (if not provided, refresh all)
        feed: Option<String>,
    },
    
    /// Show RSS-FUSE status
    Status {
        /// Check mount status for specific path
        #[arg(short, long)]
        mount_point: Option<PathBuf>,
    },
    
    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    
    /// Demo the filesystem structure without mounting
    Demo {
        /// Show detailed article content
        #[arg(long)]
        detailed: bool,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        // Initialize logging
        commands::init_logging(self.debug, self.verbose)?;
        
        match self.command {
            Commands::Init { mount_point } => {
                commands::init(mount_point).await
            }
            Commands::Mount { mount_point, daemon, allow_other, foreground, no_auto_open, file_manager } => {
                mount::mount(mount_point, daemon, allow_other, foreground, no_auto_open, file_manager, self.config).await
            }
            Commands::Unmount { mount_point, force } => {
                mount::unmount(mount_point, force).await
            }
            Commands::AddFeed { name, url } => {
                commands::add_feed(name, url, self.config).await
            }
            Commands::RemoveFeed { name } => {
                commands::remove_feed(name, self.config).await
            }
            Commands::ListFeeds => {
                commands::list_feeds(self.config).await
            }
            Commands::Refresh { feed } => {
                commands::refresh(feed, self.config).await
            }
            Commands::Status { mount_point } => {
                commands::status(mount_point).await
            }
            Commands::Completions { shell } => {
                commands::generate_completions(shell);
                Ok(())
            }
            Commands::Demo { detailed } => {
                commands::demo_filesystem(detailed, self.config).await
            }
        }
    }
}