# RSS-FUSE

A Rust-based RSS reader that uses FUSE to expose RSS feeds as a filesystem, enabling seamless navigation with TUI file managers like Yazi and Ranger.

## Overview

RSS-FUSE mounts RSS feeds as directories in your filesystem, where each feed becomes a folder and each article becomes a readable text file. This allows you to browse, search, and read RSS content using any file manager or command-line tool.

## Features

- **FUSE Filesystem**: Mount RSS feeds as directories and files
- **TUI Integration**: Perfect for Yazi, Ranger, and other file managers
- **Multiple Feed Formats**: Support for RSS 2.0 and Atom feeds
- **Intelligent Caching**: Local storage with configurable refresh intervals
- **Real-time Updates**: Automatic feed synchronization
- **Article Content**: Full article text extraction when available
- **Configuration Management**: TOML-based feed configuration

## Filesystem Structure

```
/mount/point/
├── hacker-news/
│   ├── show-hn-new-rust-crate-for-async.txt
│   ├── ask-hn-best-practices-for-microservices.txt
│   └── ...
├── rust-blog/
│   ├── announcing-rust-1-75.txt
│   ├── async-rust-in-2024.txt
│   └── ...
├── tech-crunch/
│   └── ...
└── .rss-fuse/
    ├── config.toml
    ├── cache/
    └── logs/
```

## Quick Start

1. **Install Dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install libfuse-dev
   
   # macOS
   brew install macfuse
   ```

2. **Build and Install**
   ```bash
   cargo build --release
   sudo cp target/release/rss-fuse /usr/local/bin/
   ```

3. **Configure Feeds**
   ```bash
   mkdir ~/rss-mount
   rss-fuse init ~/rss-mount
   # Edit ~/.config/rss-fuse/config.toml
   ```

4. **Mount Filesystem**
   ```bash
   rss-fuse mount ~/rss-mount
   ```

5. **Browse with Your Favorite File Manager**
   ```bash
   yazi ~/rss-mount
   # or
   ranger ~/rss-mount
   ```

## Configuration

Edit `~/.config/rss-fuse/config.toml`:

```toml
[feeds]
"hacker-news" = "https://hnrss.org/frontpage"
"rust-blog" = "https://blog.rust-lang.org/feed.xml"
"tech-crunch" = "https://techcrunch.com/feed/"

[settings]
refresh_interval = 300  # seconds
cache_duration = 3600   # seconds
max_articles = 100      # per feed
article_content = true  # fetch full content
```

## Usage with TUI File Managers

### Yazi
```bash
yazi ~/rss-mount
# Use j/k to navigate, Enter to read articles
```

### Ranger
```bash
ranger ~/rss-mount
# Browse feeds and preview articles with i
```

### Command Line
```bash
# List all feeds
ls ~/rss-mount

# Read an article
cat ~/rss-mount/hacker-news/latest-article.txt

# Search articles
grep -r "rust" ~/rss-mount/
```

## Commands

```bash
# Initialize configuration
rss-fuse init <mount-point>

# Mount filesystem
rss-fuse mount <mount-point> [options]

# Refresh feeds manually
rss-fuse refresh

# Add a new feed
rss-fuse add-feed <name> <url>

# Remove a feed
rss-fuse remove-feed <name>

# Show status
rss-fuse status

# Unmount
rss-fuse unmount <mount-point>
```

## Article Format

Each article file contains:
```
Title: Article Title Here
Author: Author Name
Published: 2024-01-15T10:30:00Z
Link: https://original-article-url.com
Tags: rust, programming, tutorial

---

Article content here...
```

## Development

```bash
# Clone repository
git clone <repository-url>
cd rss-fuse

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- mount ~/test-mount

# Development mount (auto-reload)
cargo watch -x "run -- mount ~/test-mount"
```

## Dependencies

- **fuser**: FUSE bindings for Rust
- **feed-rs**: RSS/Atom feed parsing
- **tokio**: Async runtime
- **reqwest**: HTTP client
- **serde**: Serialization
- **toml**: Configuration parsing
- **clap**: Command-line interface
- **tracing**: Logging

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Troubleshooting

See [docs/troubleshooting.md](docs/troubleshooting.md) for common issues and solutions.