# RSS-FUSE Architecture

## Overview

RSS-FUSE is built around a layered architecture that separates concerns between filesystem operations, feed management, and data persistence.

## Core Components

### 1. FUSE Filesystem Layer (`src/fuse/`)

The FUSE layer implements the filesystem interface and handles all file operations.

```
src/fuse/
├── mod.rs           # FUSE trait implementations
├── filesystem.rs    # Main filesystem struct
├── operations.rs    # File/directory operations
└── inode.rs        # Inode management
```

**Key Responsibilities:**
- Mount/unmount operations
- File and directory metadata
- Read operations for articles
- Directory listings for feeds
- Inode allocation and management

**Filesystem Structure:**
```
/mount/point/
├── {feed-name}/              # Feed directory
│   ├── {article-slug}.txt    # Article files
│   └── .metadata/            # Feed metadata
│       ├── info.json         # Feed information
│       └── last_updated      # Last refresh timestamp
├── .rss-fuse/                # System directory
│   ├── config.toml           # Configuration file
│   ├── cache/                # Article cache
│   └── logs/                 # System logs
└── README                    # Usage instructions
```

### 2. Feed Management (`src/feed/`)

Handles RSS/Atom feed parsing, fetching, and content management.

```
src/feed/
├── mod.rs           # Public API
├── parser.rs        # RSS/Atom parsing
├── fetcher.rs       # HTTP feed fetching
├── manager.rs       # Feed lifecycle management
├── content.rs       # Article content extraction
└── cache.rs         # Feed caching logic
```

**Key Features:**
- Support for RSS 2.0 and Atom 1.0
- Concurrent feed fetching
- Content extraction from HTML
- Duplicate article detection
- Configurable refresh intervals

### 3. Storage Layer (`src/storage/`)

Manages data persistence, caching, and configuration.

```
src/storage/
├── mod.rs           # Storage abstraction
├── cache.rs         # Article cache implementation
├── config.rs        # Configuration management
└── database.rs      # Metadata storage (SQLite)
```

**Caching Strategy:**
- **Memory Cache**: Recently accessed articles
- **Disk Cache**: All fetched articles with TTL
- **Database**: Article metadata and feed state

### 4. Command Line Interface (`src/cli/`)

Provides the user interface for configuration and control.

```
src/cli/
├── mod.rs           # CLI entry point
├── commands.rs      # Command implementations
├── mount.rs         # Mount/unmount logic
└── config.rs        # Configuration commands
```

## Data Flow

### 1. Feed Refresh Cycle

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│   Timer     │───▶│   Fetcher    │───▶│   Parser    │
│  (300s)     │    │ (HTTP Client)│    │ (feed-rs)   │
└─────────────┘    └──────────────┘    └─────────────┘
                                              │
                                              ▼
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│   Cache     │◀───│   Manager    │◀───│  Articles   │
│ (Disk/DB)   │    │ (Lifecycle)  │    │   (Vec)     │
└─────────────┘    └──────────────┘    └─────────────┘
```

### 2. File System Operations

```
User Request (ls, cat, etc.)
         │
         ▼
┌─────────────────┐
│  FUSE Layer     │
│ (filesystem.rs) │
└─────────────────┘
         │
         ▼
┌─────────────────┐    ┌─────────────────┐
│   Inode Mgmt    │───▶│   Cache Lookup  │
│  (inode.rs)     │    │   (cache.rs)    │
└─────────────────┘    └─────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│   Response      │◀───│   Content       │
│   (Metadata)    │    │   (Article)     │
└─────────────────┘    └─────────────────┘
```

## Threading Model

RSS-FUSE uses a multi-threaded architecture with async/await:

- **Main Thread**: FUSE operations (blocking)
- **Async Runtime**: Feed fetching and parsing
- **Background Tasks**: Cache cleanup, feed refresh
- **Worker Threads**: Content extraction, file I/O

```rust
┌─────────────────┐
│   Main Thread   │  FUSE callbacks (sync)
│   (FUSE Loop)   │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  Tokio Runtime  │  Feed operations (async)
│ (async/await)   │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│ Background Pool │  Heavy operations
│  (rayon/tokio)  │
└─────────────────┘
```

## Configuration System

### Configuration Hierarchy

1. **System**: `/etc/rss-fuse/config.toml`
2. **User**: `~/.config/rss-fuse/config.toml`
3. **Local**: `./rss-fuse.toml`
4. **Environment**: `RSS_FUSE_*` variables
5. **Command Line**: CLI arguments

### Configuration Schema

```toml
[feeds]
# Feed name = URL mapping
"hacker-news" = "https://hnrss.org/frontpage"

[settings]
refresh_interval = 300      # seconds
cache_duration = 3600       # seconds
max_articles = 100          # per feed
concurrent_fetches = 5      # parallel downloads
article_content = true      # extract full content

[filesystem]
mount_options = ["allow_other", "auto_unmount"]
file_permissions = 644
dir_permissions = 755

[logging]
level = "info"
file = "~/.local/share/rss-fuse/logs/rss-fuse.log"
```

## Error Handling

### Error Types

1. **Feed Errors**: Network timeouts, invalid XML, 404s
2. **FUSE Errors**: Permission denied, file not found
3. **Storage Errors**: Disk full, database corruption
4. **Configuration Errors**: Invalid TOML, missing feeds

### Error Recovery

- **Graceful Degradation**: Show cached content when feeds fail
- **Retry Logic**: Exponential backoff for network errors
- **User Feedback**: Log errors to filesystem (`.rss-fuse/errors/`)
- **Health Checks**: Monitor feed status and report issues

## Performance Considerations

### Memory Management
- **Lazy Loading**: Load article content on demand
- **LRU Cache**: Evict least recently used articles
- **Streaming**: Process large feeds without loading everything

### I/O Optimization
- **Batch Operations**: Group database writes
- **Async I/O**: Non-blocking network and file operations
- **Compression**: Gzip article content in cache

### FUSE Optimization
- **Attribute Caching**: Cache file metadata
- **Read-ahead**: Prefetch directory contents
- **Connection Pooling**: Reuse HTTP connections

## Security Considerations

### Input Validation
- **URL Sanitization**: Validate feed URLs
- **XML Parsing**: Prevent XXE attacks
- **File Paths**: Sanitize article titles for safe filenames

### Access Control
- **User Permissions**: Respect filesystem permissions
- **Network Limits**: Rate limit HTTP requests
- **Resource Limits**: Prevent DoS via large feeds

### Data Privacy
- **No Tracking**: Don't send user information to feed servers
- **Local Storage**: All data stays on user's machine
- **Secure Defaults**: Conservative configuration defaults

## Testing Strategy

### Unit Tests
- Feed parsing logic
- Cache operations
- Configuration loading
- URL validation

### Integration Tests
- FUSE operations
- End-to-end feed processing
- Error scenarios
- Performance benchmarks

### System Tests
- Real feed compatibility
- File manager integration
- Long-running stability
- Resource usage monitoring