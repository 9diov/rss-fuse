# Installation Guide

## Prerequisites

### System Dependencies

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install -y \
    libfuse-dev \
    pkg-config \
    build-essential \
    curl
```

#### Linux (CentOS/RHEL/Fedora)
```bash
# CentOS/RHEL
sudo yum install -y fuse-devel pkgconfig gcc curl
# or Fedora
sudo dnf install -y fuse-devel pkgconfig gcc curl
```

#### macOS
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install macfuse pkg-config
```

#### Arch Linux
```bash
sudo pacman -S fuse2 pkgconf base-devel curl
```

### Rust Installation

If you don't have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

Ensure you have a recent version:
```bash
rustup update
rustc --version  # Should be 1.70.0 or later
```

## Installation Methods

### Method 1: From Source (Recommended)

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-username/rss-fuse.git
   cd rss-fuse
   ```

2. **Build the project**
   ```bash
   cargo build --release
   ```

3. **Install system-wide**
   ```bash
   sudo cp target/release/rss-fuse /usr/local/bin/
   sudo chmod +x /usr/local/bin/rss-fuse
   ```

4. **Verify installation**
   ```bash
   rss-fuse --version
   ```

### Method 2: Using Cargo Install

```bash
cargo install rss-fuse
```

### Method 3: Download Pre-built Binaries

Download from the [releases page](https://github.com/your-username/rss-fuse/releases):

```bash
# Linux x86_64
wget https://github.com/your-username/rss-fuse/releases/latest/download/rss-fuse-linux-x86_64.tar.gz
tar -xzf rss-fuse-linux-x86_64.tar.gz
sudo mv rss-fuse /usr/local/bin/

# macOS (Intel)
wget https://github.com/your-username/rss-fuse/releases/latest/download/rss-fuse-macos-x86_64.tar.gz
tar -xzf rss-fuse-macos-x86_64.tar.gz
sudo mv rss-fuse /usr/local/bin/

# macOS (Apple Silicon)
wget https://github.com/your-username/rss-fuse/releases/latest/download/rss-fuse-macos-aarch64.tar.gz
tar -xzf rss-fuse-macos-aarch64.tar.gz
sudo mv rss-fuse /usr/local/bin/
```

## Configuration Setup

### 1. Initialize Configuration

```bash
# Create mount point
mkdir ~/rss-mount

# Initialize configuration
rss-fuse init ~/rss-mount
```

This creates:
- `~/.config/rss-fuse/config.toml` - Main configuration
- `~/.local/share/rss-fuse/cache/` - Cache directory
- `~/.local/share/rss-fuse/logs/` - Log directory

### 2. Edit Configuration

Edit `~/.config/rss-fuse/config.toml`:

```toml
[feeds]
"hacker-news" = "https://hnrss.org/frontpage"
"rust-blog" = "https://blog.rust-lang.org/feed.xml"
"lobsters" = "https://lobste.rs/rss"

[settings]
refresh_interval = 300  # 5 minutes
cache_duration = 3600   # 1 hour
max_articles = 100
concurrent_fetches = 5
article_content = true

[filesystem]
file_permissions = 644
dir_permissions = 755
allow_other = false
auto_unmount = true

[logging]
level = "info"
file = "~/.local/share/rss-fuse/logs/rss-fuse.log"
```

## First Run

### 1. Mount the Filesystem

```bash
rss-fuse mount ~/rss-mount
```

The process will run in the foreground. To run in background:
```bash
rss-fuse mount ~/rss-mount --daemon
```

### 2. Test the Mount

In another terminal:
```bash
# List feeds
ls ~/rss-mount/

# Browse a feed
ls ~/rss-mount/hacker-news/

# Read an article
cat ~/rss-mount/hacker-news/some-article.txt
```

### 3. Use with File Managers

```bash
# Yazi
yazi ~/rss-mount/

# Ranger
ranger ~/rss-mount/

# nnn
nnn ~/rss-mount/
```

## Troubleshooting

### Common Issues

#### 1. Permission Denied Errors

```bash
# Add user to fuse group (Linux)
sudo usermod -a -G fuse $USER
# Log out and back in

# Or use allow_other option
echo "user_allow_other" | sudo tee -a /etc/fuse.conf
```

#### 2. FUSE Module Not Loaded

```bash
# Linux
sudo modprobe fuse

# Check if loaded
lsmod | grep fuse
```

#### 3. macOS: Operation Not Permitted

```bash
# Install macFUSE and restart
brew install --cask macfuse
# Restart your Mac
# Allow in System Preferences > Security & Privacy
```

#### 4. Build Errors

```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release

# Check system dependencies
pkg-config --libs fuse  # Should not error
```

#### 5. Network Issues

```bash
# Test feed URLs manually
curl -I "https://blog.rust-lang.org/feed.xml"

# Check proxy settings
export HTTP_PROXY=http://your-proxy:port
export HTTPS_PROXY=http://your-proxy:port
```

### Debug Mode

Run with debug logging:
```bash
RUST_LOG=debug rss-fuse mount ~/rss-mount
```

Check logs:
```bash
tail -f ~/.local/share/rss-fuse/logs/rss-fuse.log
```

### Cleanup

To completely remove RSS-FUSE:

```bash
# Unmount if mounted
rss-fuse unmount ~/rss-mount

# Remove binary
sudo rm /usr/local/bin/rss-fuse

# Remove configuration and data
rm -rf ~/.config/rss-fuse/
rm -rf ~/.local/share/rss-fuse/

# Remove mount point
rmdir ~/rss-mount
```

## System Service (Optional)

### Linux (systemd)

Create `/etc/systemd/system/rss-fuse.service`:

```ini
[Unit]
Description=RSS FUSE Filesystem
After=network.target

[Service]
Type=simple
User=%i
Environment=RUST_LOG=info
ExecStart=/usr/local/bin/rss-fuse mount /home/%i/rss-mount --daemon
ExecStop=/usr/local/bin/rss-fuse unmount /home/%i/rss-mount
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable rss-fuse@$USER
sudo systemctl start rss-fuse@$USER
```

### macOS (launchd)

Create `~/Library/LaunchAgents/com.rss-fuse.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.rss-fuse</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/rss-fuse</string>
        <string>mount</string>
        <string>/Users/YOUR_USERNAME/rss-mount</string>
        <string>--daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load and start:
```bash
launchctl load ~/Library/LaunchAgents/com.rss-fuse.plist
launchctl start com.rss-fuse
```

## Performance Tuning

### For Many Feeds (>50)

```toml
[settings]
concurrent_fetches = 10    # Increase parallel downloads
cache_duration = 7200      # Cache longer
refresh_interval = 600     # Refresh less frequently

[filesystem]
allow_other = true         # Better performance with some file managers
```

### For Slow Networks

```toml
[settings]
concurrent_fetches = 2     # Reduce bandwidth usage
timeout = 30               # Increase timeout
retry_attempts = 3         # More retries
```

### For Large Articles

```toml
[settings]
article_content = false    # Disable full content extraction
max_article_size = 1048576 # 1MB limit
```

## Updating

### From Source
```bash
cd rss-fuse
git pull origin main
cargo build --release
sudo cp target/release/rss-fuse /usr/local/bin/
```

### Using Cargo
```bash
cargo install rss-fuse --force
```

Always check the [changelog](CHANGELOG.md) for breaking changes before updating.