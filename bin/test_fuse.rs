use rss_fuse::fuse::{FuseOperations, MountOptions};
use rss_fuse::feed::{Feed, Article, ParsedArticle, FeedStatus};
use rss_fuse::error::Result;
use chrono::Utc;
// use std::sync::Arc;

fn create_test_feed(name: &str, article_count: usize) -> Feed {
    let mut articles = Vec::new();
    
    for i in 0..article_count {
        let parsed_article = ParsedArticle {
            title: format!("Article {} from {}", i + 1, name),
            link: format!("https://example.com/{}/article-{}", name, i + 1),
            description: Some(format!("This is article {} from the {} feed", i + 1, name)),
            content: Some(format!("Full content of article {} from {}. This is a longer text that would appear when reading the file.", i + 1, name)),
            author: Some(format!("Author {}", i + 1)),
            published: Some(Utc::now()),
            guid: Some(format!("{}-article-{}", name, i + 1)),
            categories: vec![name.to_string(), "test".to_string()],
        };
        
        let article = Article::new(parsed_article, name);
        articles.push(article);
    }
    
    Feed {
        name: name.to_string(),
        url: format!("https://example.com/{}/feed.xml", name),
        title: Some(format!("{} Feed", name.to_uppercase())),
        description: Some(format!("A test feed for {}", name)),
        last_updated: Some(Utc::now()),
        articles,
        status: FeedStatus::Active,
    }
}

fn test_filesystem_operations() -> Result<()> {
    println!("🔧 Creating FUSE operations...");
    let fuse_ops = FuseOperations::new();
    
    // Test filesystem statistics
    let initial_stats = fuse_ops.get_stats();
    println!("📊 Initial stats: {} inodes, {} feeds", 
             initial_stats.total_inodes, initial_stats.feeds_count);
    
    // Test adding feeds
    println!("\n📝 Adding test feeds...");
    let tech_feed = create_test_feed("tech-news", 3);
    let science_feed = create_test_feed("science-daily", 2);
    let blog_feed = create_test_feed("personal-blog", 1);
    
    fuse_ops.filesystem.add_feed(tech_feed)?;
    fuse_ops.filesystem.add_feed(science_feed)?;
    fuse_ops.filesystem.add_feed(blog_feed)?;
    
    // Test updated statistics
    let updated_stats = fuse_ops.get_stats();
    println!("📊 Updated stats: {} inodes, {} feeds", 
             updated_stats.total_inodes, updated_stats.feeds_count);
    
    // Test filesystem structure
    println!("\n🗂️  Testing filesystem structure...");
    
    // Test root directory
    let root_node = fuse_ops.filesystem.get_node(1).unwrap();
    println!("Root directory: {} (inode: {})", root_node.name, root_node.ino);
    
    let root_children = fuse_ops.filesystem.list_children(1);
    println!("Root has {} children:", root_children.len());
    
    for child in &root_children {
        println!("  - {} ({})", child.name, 
                 if child.is_directory() { "directory" } else { "file" });
        
        if child.is_directory() && !child.name.starts_with('.') {
            // List articles in feed directory
            let articles = fuse_ops.filesystem.list_children(child.ino);
            println!("    {} has {} articles:", child.name, articles.len());
            for article in &articles {
                println!("      - {} ({} bytes)", article.name, article.size);
            }
        }
    }
    
    // Test reading article content
    println!("\n📖 Testing article content reading...");
    for child in &root_children {
        if child.is_directory() && !child.name.starts_with('.') {
            let articles = fuse_ops.filesystem.list_children(child.ino);
            if let Some(first_article) = articles.first() {
                if let Some(content) = fuse_ops.filesystem.get_article_content(first_article.ino) {
                    println!("📄 Content preview for '{}':", first_article.name);
                    let preview = if content.len() > 200 {
                        format!("{}...", &content[..200])
                    } else {
                        content
                    };
                    println!("   {}", preview.replace('\n', "\n   "));
                    println!();
                }
            }
        }
    }
    
    // Test config update
    println!("⚙️  Testing config management...");
    let config_content = r#"[feeds]
"tech-news" = "https://example.com/tech-news/feed.xml"
"science-daily" = "https://example.com/science-daily/feed.xml"
"personal-blog" = "https://example.com/personal-blog/feed.xml"

[settings]
refresh_interval = 300
cache_duration = 3600
max_articles = 100
article_content = true
"#.to_string();
    
    fuse_ops.filesystem.update_config(config_content);
    println!("✅ Config updated successfully");
    
    // Test feed removal
    println!("\n🗑️  Testing feed removal...");
    fuse_ops.filesystem.remove_feed("personal-blog")?;
    
    let final_stats = fuse_ops.get_stats();
    println!("📊 Final stats: {} inodes, {} feeds", 
             final_stats.total_inodes, final_stats.feeds_count);
    
    println!("\n✅ All FUSE filesystem tests completed successfully!");
    
    Ok(())
}

fn test_mount_operations() -> Result<()> {
    println!("\n🔧 Testing mount operations...");
    
    let fuse_ops = FuseOperations::new();
    
    // Test mount options
    let mut options = MountOptions::default();
    options.allow_other = false;
    options.read_only = true;
    
    println!("📋 Mount options: {:?}", options);
    
    // Note: We can't actually mount in a test environment,
    // but we can test the validation logic
    
    // Test with a temporary directory
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| rss_fuse::error::Error::Io(e))?;
    
    println!("🗂️  Test mount point: {}", temp_dir.path().display());
    
    // Test mount point validation
    match fuse_ops.validate_mount_point(temp_dir.path()) {
        Ok(()) => println!("✅ Mount point validation passed"),
        Err(e) => println!("❌ Mount point validation failed: {}", e),
    }
    
    // Test is_mounted check
    let is_mounted = fuse_ops.is_mounted(temp_dir.path());
    println!("📍 Is mounted: {}", is_mounted);
    
    println!("✅ Mount operation tests completed!");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 RSS-FUSE Filesystem Testing");
    println!("==============================\n");
    
    // Test filesystem operations
    test_filesystem_operations()?;
    
    // Test mount operations
    test_mount_operations()?;
    
    println!("\n🎉 All tests completed successfully!");
    
    Ok(())
}