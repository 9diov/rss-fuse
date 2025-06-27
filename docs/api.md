# RSS-FUSE API Documentation

## Feed Management API

### Core Structs

#### `Feed`
```rust
pub struct Feed {
    pub name: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
    pub articles: Vec<Article>,
    pub status: FeedStatus,
}

pub enum FeedStatus {
    Active,
    Error(String),
    Updating,
    Disabled,
}
```

#### `Article`
```rust
pub struct Article {
    pub id: String,
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub read: bool,
}
```

### Feed Operations

#### `FeedManager`
```rust
impl FeedManager {
    /// Create a new feed manager
    pub fn new(config: Config) -> Result<Self, Error>;
    
    /// Add a new feed
    pub async fn add_feed(&mut self, name: String, url: String) -> Result<(), Error>;
    
    /// Remove a feed
    pub async fn remove_feed(&mut self, name: &str) -> Result<(), Error>;
    
    /// Refresh a specific feed
    pub async fn refresh_feed(&mut self, name: &str) -> Result<(), Error>;
    
    /// Refresh all feeds
    pub async fn refresh_all(&mut self) -> Result<Vec<FeedResult>, Error>;
    
    /// Get feed by name
    pub fn get_feed(&self, name: &str) -> Option<&Feed>;
    
    /// List all feeds
    pub fn list_feeds(&self) -> Vec<&Feed>;
    
    /// Get article by feed and ID
    pub fn get_article(&self, feed_name: &str, article_id: &str) -> Option<&Article>;
}

pub struct FeedResult {
    pub feed_name: String,
    pub success: bool,
    pub error: Option<String>,
    pub articles_added: usize,
    pub articles_updated: usize,
}
```

#### `FeedFetcher`
```rust
impl FeedFetcher {
    /// Create a new fetcher with HTTP client
    pub fn new() -> Self;
    
    /// Fetch and parse a feed from URL
    pub async fn fetch(&self, url: &str) -> Result<ParsedFeed, FetchError>;
    
    /// Fetch with custom headers
    pub async fn fetch_with_headers(
        &self, 
        url: &str, 
        headers: HeaderMap
    ) -> Result<ParsedFeed, FetchError>;
}

pub struct ParsedFeed {
    pub title: String,
    pub description: Option<String>,
    pub link: Option<String>,
    pub last_build_date: Option<DateTime<Utc>>,
    pub articles: Vec<ParsedArticle>,
}

pub struct ParsedArticle {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub guid: Option<String>,
    pub categories: Vec<String>,
}
```

## FUSE Filesystem API

### Filesystem Operations

#### `RssFuseFilesystem`
```rust
impl Filesystem for RssFuseFilesystem {
    /// Initialize filesystem
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), c_int>;
    
    /// Get file attributes
    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr);
    
    /// Read directory contents
    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        reply: ReplyDirectory,
    );
    
    /// Open file
    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen);
    
    /// Read file contents
    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    );
}
```

#### `InodeManager`
```rust
impl InodeManager {
    /// Allocate new inode
    pub fn allocate(&mut self) -> u64;
    
    /// Register inode with path
    pub fn register(&mut self, ino: u64, path: PathBuf, node_type: NodeType);
    
    /// Get inode for path
    pub fn get_inode(&self, path: &Path) -> Option<u64>;
    
    /// Get path for inode
    pub fn get_path(&self, ino: u64) -> Option<&Path>;
    
    /// Get node type
    pub fn get_node_type(&self, ino: u64) -> Option<NodeType>;
}

pub enum NodeType {
    Root,
    Feed(String),
    Article(String, String), // feed_name, article_id
    SystemDir,
    ConfigFile,
    LogFile,
}
```

## Caching API

### Cache Management

#### `ArticleCache`
```rust
impl ArticleCache {
    /// Create new cache with capacity
    pub fn new(capacity: usize, ttl: Duration) -> Self;
    
    /// Store article content
    pub fn store(&mut self, key: &str, content: String) -> Result<(), CacheError>;
    
    /// Retrieve article content
    pub fn get(&mut self, key: &str) -> Option<String>;
    
    /// Check if article exists
    pub fn contains(&self, key: &str) -> bool;
    
    /// Remove article from cache
    pub fn remove(&mut self, key: &str) -> Option<String>;
    
    /// Clear all cached articles
    pub fn clear(&mut self);
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats;
    
    /// Cleanup expired entries
    pub fn cleanup(&mut self) -> usize;
}

pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub memory_usage: usize,
    pub hit_rate: f64,
}
```

#### `PersistentCache`
```rust
impl PersistentCache {
    /// Create cache with disk backing
    pub fn new(cache_dir: PathBuf) -> Result<Self, CacheError>;
    
    /// Store article to disk
    pub async fn store_article(
        &self, 
        feed_name: &str, 
        article_id: &str, 
        content: &str
    ) -> Result<(), CacheError>;
    
    /// Load article from disk
    pub async fn load_article(
        &self, 
        feed_name: &str, 
        article_id: &str
    ) -> Result<Option<String>, CacheError>;
    
    /// Store feed metadata
    pub async fn store_feed_meta(
        &self, 
        feed_name: &str, 
        metadata: &FeedMetadata
    ) -> Result<(), CacheError>;
    
    /// Load feed metadata
    pub async fn load_feed_meta(
        &self, 
        feed_name: &str
    ) -> Result<Option<FeedMetadata>, CacheError>;
}
```

## Configuration API

### Configuration Management

#### `Config`
```rust
pub struct Config {
    pub feeds: HashMap<String, String>,
    pub settings: Settings,
    pub filesystem: FilesystemConfig,
    pub logging: LoggingConfig,
}

pub struct Settings {
    pub refresh_interval: u64,
    pub cache_duration: u64,
    pub max_articles: usize,
    pub concurrent_fetches: usize,
    pub article_content: bool,
    pub user_agent: String,
}

pub struct FilesystemConfig {
    pub mount_options: Vec<String>,
    pub file_permissions: u32,
    pub dir_permissions: u32,
    pub allow_other: bool,
    pub auto_unmount: bool,
}

impl Config {
    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError>;
    
    /// Load with environment overrides
    pub fn load_with_env<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError>;
    
    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError>;
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError>;
    
    /// Get default configuration
    pub fn default() -> Self;
}
```

## Content Extraction API

### Article Content Processing

#### `ContentExtractor`
```rust
impl ContentExtractor {
    /// Create new extractor
    pub fn new() -> Self;
    
    /// Extract readable content from HTML
    pub async fn extract_content(&self, url: &str, html: &str) -> Result<String, ExtractionError>;
    
    /// Extract with custom selectors
    pub async fn extract_with_selectors(
        &self, 
        html: &str, 
        selectors: &ContentSelectors
    ) -> Result<String, ExtractionError>;
    
    /// Clean and format text
    pub fn clean_text(&self, text: &str) -> String;
    
    /// Convert HTML to markdown
    pub fn html_to_markdown(&self, html: &str) -> String;
}

pub struct ContentSelectors {
    pub article: Vec<String>,
    pub content: Vec<String>,
    pub remove: Vec<String>,
}
```

## Error Types

### Error Handling

```rust
#[derive(Debug, Error)]
pub enum RssFuseError {
    #[error("Feed error: {0}")]
    Feed(#[from] FeedError),
    
    #[error("FUSE error: {0}")]
    Fuse(#[from] FuseError),
    
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),
    
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("Invalid feed URL: {url}")]
    InvalidUrl { url: String },
    
    #[error("Feed not found: {name}")]
    NotFound { name: String },
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Network timeout")]
    Timeout,
    
    #[error("Too many redirects")]
    TooManyRedirects,
}
```

## Usage Examples

### Basic Feed Management
```rust
use rss_fuse::{Config, FeedManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load("config.toml")?;
    let mut manager = FeedManager::new(config)?;
    
    // Add a feed
    manager.add_feed(
        "rust-blog".to_string(),
        "https://blog.rust-lang.org/feed.xml".to_string()
    ).await?;
    
    // Refresh all feeds
    let results = manager.refresh_all().await?;
    for result in results {
        println!("Feed {}: {} articles", result.feed_name, result.articles_added);
    }
    
    Ok(())
}
```

### FUSE Filesystem Usage
```rust
use rss_fuse::{RssFuseFilesystem, Config};
use fuser::MountOption;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load("config.toml")?;
    let fs = RssFuseFilesystem::new(config)?;
    
    let options = vec![
        MountOption::RO,
        MountOption::FSName("rss-fuse".to_string()),
    ];
    
    fuser::mount2(fs, "/tmp/rss-mount", &options)?;
    
    Ok(())
}
```

### Content Extraction
```rust
use rss_fuse::ContentExtractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = ContentExtractor::new();
    
    let html = "<html><body><article>Content here</article></body></html>";
    let content = extractor.extract_content("", html).await?;
    let markdown = extractor.html_to_markdown(&content);
    
    println!("Extracted content: {}", markdown);
    
    Ok(())
}
```