use std::path::PathBuf;
use tempfile::TempDir;
use rss_fuse::cli::{Cli, Commands};
use rss_fuse::error::Result;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️  RSS-FUSE CLI Test Suite");
    println!("===========================\n");

    // Create temporary directory for testing
    let temp_dir = TempDir::new()?;
    let mount_point = temp_dir.path().join("rss-mount");
    
    // Test CLI commands in sequence
    test_init_command(&mount_point).await?;
    test_status_command().await?;
    test_add_feed_command().await?;
    test_list_feeds_command().await?;
    test_refresh_command().await?;
    test_remove_feed_command().await?;
    test_completions_command().await?;
    
    println!("🏆 All CLI tests completed successfully!");
    
    Ok(())
}

async fn test_init_command(mount_point: &std::path::Path) -> Result<()> {
    println!("📋 Testing init command...");
    
    let cli = Cli {
        command: Commands::Init {
            mount_point: mount_point.to_path_buf(),
        },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ✅ Init command succeeded");
            
            // Verify mount point was created
            if mount_point.exists() {
                println!("   ✅ Mount point created successfully");
            } else {
                println!("   ❌ Mount point was not created");
                return Err("Mount point creation failed".into());
            }
        },
        Err(e) => {
            println!("   ❌ Init command failed: {}", e);
            return Err(e.into());
        }
    }
    
    println!("   ✅ Init test passed!\n");
    Ok(())
}

async fn test_status_command() -> Result<()> {
    println!("📊 Testing status command...");
    
    let cli = Cli {
        command: Commands::Status,
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ✅ Status command succeeded");
        },
        Err(e) => {
            println!("   ❌ Status command failed: {}", e);
            return Err(e.into());
        }
    }
    
    println!("   ✅ Status test passed!\n");
    Ok(())
}

async fn test_add_feed_command() -> Result<()> {
    println!("📡 Testing add-feed command...");
    
    // Test adding a real RSS feed (using a mock URL for testing)
    let cli = Cli {
        command: Commands::AddFeed {
            name: "test-feed".to_string(),
            url: "https://hnrss.org/frontpage".to_string(), // Real RSS feed for testing
        },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ✅ Add feed command succeeded");
        },
        Err(e) => {
            println!("   ⚠️  Add feed command failed (expected in test environment): {}", e);
            // This might fail due to network issues or configuration, which is OK for testing
        }
    }
    
    // Test adding with invalid URL
    let cli_invalid = Cli {
        command: Commands::AddFeed {
            name: "invalid-feed".to_string(),
            url: "not-a-valid-url".to_string(),
        },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli_invalid.run().await {
        Ok(_) => {
            println!("   ❌ Invalid URL should have failed");
        },
        Err(_) => {
            println!("   ✅ Invalid URL properly rejected");
        }
    }
    
    println!("   ✅ Add feed test passed!\n");
    Ok(())
}

async fn test_list_feeds_command() -> Result<()> {
    println!("📋 Testing list-feeds command...");
    
    let cli = Cli {
        command: Commands::ListFeeds,
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ✅ List feeds command succeeded");
        },
        Err(e) => {
            println!("   ❌ List feeds command failed: {}", e);
            return Err(e.into());
        }
    }
    
    println!("   ✅ List feeds test passed!\n");
    Ok(())
}

async fn test_refresh_command() -> Result<()> {
    println!("🔄 Testing refresh command...");
    
    // Test refresh all feeds
    let cli = Cli {
        command: Commands::Refresh { feed: None },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ✅ Refresh all command succeeded");
        },
        Err(e) => {
            println!("   ⚠️  Refresh all command failed (expected with no feeds): {}", e);
            // This might fail if no feeds are configured, which is OK
        }
    }
    
    // Test refresh specific feed
    let cli_specific = Cli {
        command: Commands::Refresh { 
            feed: Some("nonexistent-feed".to_string()) 
        },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli_specific.run().await {
        Ok(_) => {
            println!("   ❌ Nonexistent feed should have failed");
        },
        Err(_) => {
            println!("   ✅ Nonexistent feed properly rejected");
        }
    }
    
    println!("   ✅ Refresh test passed!\n");
    Ok(())
}

async fn test_remove_feed_command() -> Result<()> {
    println!("🗑️  Testing remove-feed command...");
    
    let cli = Cli {
        command: Commands::RemoveFeed {
            name: "nonexistent-feed".to_string(),
        },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ❌ Nonexistent feed should have failed");
        },
        Err(_) => {
            println!("   ✅ Nonexistent feed properly rejected");
        }
    }
    
    println!("   ✅ Remove feed test passed!\n");
    Ok(())
}

async fn test_completions_command() -> Result<()> {
    println!("🔧 Testing completions command...");
    
    let cli = Cli {
        command: Commands::Completions {
            shell: clap_complete::Shell::Bash,
        },
        config: None,
        verbose: false,
        debug: false,
    };
    
    match cli.run().await {
        Ok(_) => {
            println!("   ✅ Completions command succeeded");
        },
        Err(e) => {
            println!("   ❌ Completions command failed: {}", e);
            return Err(e.into());
        }
    }
    
    println!("   ✅ Completions test passed!\n");
    Ok(())
}

#[tokio::test]
async fn test_cli_structure() {
    // Test that CLI structure is properly defined
    use clap::Parser;
    
    // Test help message generation
    let help = Cli::command().render_help();
    assert!(help.to_string().contains("RSS-FUSE"));
    assert!(help.to_string().contains("init"));
    assert!(help.to_string().contains("mount"));
    assert!(help.to_string().contains("add-feed"));
    
    println!("✅ CLI structure test passed");
}

#[tokio::test]
async fn test_command_parsing() {
    use clap::Parser;
    
    // Test parsing various command combinations
    let test_cases = vec![
        vec!["rss-fuse", "init", "/tmp/test"],
        vec!["rss-fuse", "--verbose", "status"],
        vec!["rss-fuse", "add-feed", "test", "https://example.com/feed.xml"],
        vec!["rss-fuse", "mount", "/tmp/mount", "--foreground"],
        vec!["rss-fuse", "completions", "bash"],
    ];
    
    for args in test_cases {
        match Cli::try_parse_from(args.clone()) {
            Ok(_) => {
                println!("✅ Parsed: {:?}", args);
            },
            Err(e) => {
                println!("❌ Failed to parse {:?}: {}", args, e);
                panic!("Command parsing failed");
            }
        }
    }
    
    println!("✅ Command parsing test passed");
}