use std::ffi::OsStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use fuser::{
    Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    FileAttr, FileType, FUSE_ROOT_ID,
};
use libc::{ENOENT, ENOTDIR, EISDIR, EINVAL};
use parking_lot::RwLock;
use tracing::{debug, warn, error};

use crate::fuse::{create_file_attr, TTL};
use crate::fuse::inode::{InodeManager, NodeType};
use crate::feed::{Feed, Article};
use crate::error::Result;

/// Main FUSE filesystem implementation for RSS-FUSE
pub struct RssFuseFilesystem {
    inode_manager: Arc<InodeManager>,
    feeds: RwLock<HashMap<String, Feed>>,
    config_content: RwLock<String>,
}

impl Clone for RssFuseFilesystem {
    fn clone(&self) -> Self {
        Self {
            inode_manager: Arc::clone(&self.inode_manager),
            feeds: RwLock::new(self.feeds.read().clone()),
            config_content: RwLock::new(self.config_content.read().clone()),
        }
    }
}

impl RssFuseFilesystem {
    pub fn new() -> Self {
        let inode_manager = Arc::new(InodeManager::new());
        
        // Create the meta structure on startup
        if let Err(e) = inode_manager.create_meta_structure() {
            error!("Failed to create meta structure: {}", e);
        }

        Self {
            inode_manager,
            feeds: RwLock::new(HashMap::new()),
            config_content: RwLock::new(String::new()),
        }
    }

    pub fn add_feed(&self, feed: Feed) -> Result<()> {
        let feed_name = feed.name.clone();
        
        // Create feed directory
        if let Err(e) = self.inode_manager.create_feed_directory(&feed_name) {
            warn!("Failed to create feed directory for {}: {}", feed_name, e);
        }

        // Add articles
        for article in &feed.articles {
            let article_arc = Arc::new(article.clone());
            if let Err(e) = self.inode_manager.create_article_file(&feed_name, article_arc) {
                warn!("Failed to create article file for {}: {}", article.title, e);
            }
        }

        // Store feed data
        self.feeds.write().insert(feed_name, feed);
        
        Ok(())
    }

    pub fn remove_feed(&self, feed_name: &str) -> Result<()> {
        // Find and remove feed directory
        if let Some(feed_node) = self.inode_manager.get_node_by_name(FUSE_ROOT_ID, feed_name) {
            // Remove all articles first
            let children = self.inode_manager.list_children(feed_node.ino);
            for child in children {
                if let Err(e) = self.inode_manager.remove_node(child.ino) {
                    warn!("Failed to remove article {}: {}", child.name, e);
                }
            }
            
            // Remove the directory itself
            if let Err(e) = self.inode_manager.remove_node(feed_node.ino) {
                warn!("Failed to remove feed directory {}: {}", feed_name, e);
            }
        }

        // Remove from feeds map
        self.feeds.write().remove(feed_name);
        
        Ok(())
    }

    pub fn get_total_inodes(&self) -> usize {
        self.inode_manager.get_total_nodes()
    }

    pub fn get_feeds_count(&self) -> usize {
        self.feeds.read().len()
    }

    pub fn get_node(&self, ino: u64) -> Option<crate::fuse::inode::VNode> {
        self.inode_manager.get_node(ino)
    }

    pub fn list_children(&self, parent_ino: u64) -> Vec<crate::fuse::inode::VNode> {
        self.inode_manager.list_children(parent_ino)
    }

    pub fn get_article_content(&self, ino: u64) -> Option<String> {
        self.inode_manager.get_article_content(ino)
    }

    pub fn get_node_by_name(&self, parent_ino: u64, name: &str) -> Option<crate::fuse::inode::VNode> {
        self.inode_manager.get_node_by_name(parent_ino, name)
    }

    pub fn update_config(&self, content: String) {
        let content_len = content.len() as u64;
        *self.config_content.write() = content;
        
        // Update the config file size
        if let Some(config_node) = self.inode_manager.get_node_by_name(1, ".rss-fuse")
            .and_then(|meta| self.inode_manager.get_node_by_name(meta.ino, "config.toml")) {
            self.inode_manager.update_node_size(config_node.ino, content_len);
        }
    }

    fn node_to_file_attr(&self, node: &crate::fuse::inode::VNode) -> FileAttr {
        let kind = node.file_type;
        let perm = match kind {
            FileType::Directory => 0o755,
            FileType::RegularFile => 0o644,
            _ => 0o644,
        };

        create_file_attr(node.ino, node.size, kind, perm)
    }

    fn lookup_node(&self, parent: u64, name: &OsStr) -> Option<crate::fuse::inode::VNode> {
        let name_str = name.to_str()?;
        self.inode_manager.get_node_by_name(parent, name_str)
    }
}

impl Filesystem for RssFuseFilesystem {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        debug!("lookup(parent: {}, name: {:?})", parent, name);

        match self.lookup_node(parent, name) {
            Some(node) => {
                let attr = self.node_to_file_attr(&node);
                reply.entry(&TTL, &attr, 0);
            }
            None => {
                debug!("lookup: not found");
                reply.error(ENOENT);
            }
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        debug!("getattr(ino: {})", ino);

        match self.inode_manager.get_node(ino) {
            Some(node) => {
                let attr = self.node_to_file_attr(&node);
                reply.attr(&TTL, &attr);
            }
            None => {
                debug!("getattr: inode {} not found", ino);
                reply.error(ENOENT);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        debug!("readdir(ino: {}, offset: {})", ino, offset);

        let node = match self.inode_manager.get_node(ino) {
            Some(node) => node,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if !node.is_directory() {
            reply.error(ENOTDIR);
            return;
        }

        let mut entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (node.parent_ino, FileType::Directory, "..".to_string()),
        ];

        // Add child entries
        let children = self.inode_manager.list_children(ino);
        for child in children {
            entries.push((child.ino, child.file_type, child.name));
        }

        // Apply offset
        for (i, (child_ino, file_type, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            debug!("  entry: {} -> {} ({})", name, child_ino, i + 1);
            
            if reply.add(child_ino, (i + 1) as i64, file_type, &name) {
                break; // Buffer is full
            }
        }

        reply.ok();
    }

    fn open(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
        debug!("open(ino: {}, flags: {})", ino, flags);

        let node = match self.inode_manager.get_node(ino) {
            Some(node) => node,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if node.is_directory() {
            reply.error(EISDIR);
            return;
        }

        // For now, we'll allow all opens and use the inode as file handle
        reply.opened(ino, 0);
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        debug!("read(ino: {}, offset: {}, size: {})", ino, offset, size);

        let node = match self.inode_manager.get_node(ino) {
            Some(node) => node,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if node.is_directory() {
            reply.error(EISDIR);
            return;
        }

        let content = match &node.node_type {
            NodeType::ArticleFile(_, _) => {
                match self.inode_manager.get_article_content(ino) {
                    Some(content) => content,
                    None => {
                        error!("Failed to get article content for inode {}", ino);
                        reply.error(ENOENT);
                        return;
                    }
                }
            }
            NodeType::ConfigFile => {
                self.config_content.read().clone()
            }
            _ => {
                warn!("Attempted to read unsupported file type: {:?}", node.node_type);
                reply.error(EINVAL);
                return;
            }
        };

        let content_bytes = content.as_bytes();
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, content_bytes.len());

        if start >= content_bytes.len() {
            reply.data(&[]);
            return;
        }

        let data = &content_bytes[start..end];
        reply.data(data);
    }

    fn release(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!("release(ino: {})", ino);
        reply.ok();
    }
}

impl Default for RssFuseFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{Article, ParsedArticle, FeedStatus};
    use chrono::Utc;

    fn create_test_feed() -> Feed {
        let parsed_article = ParsedArticle {
            title: "Test Article".to_string(),
            link: "https://example.com/test".to_string(),
            description: Some("Test description".to_string()),
            content: None,
            author: Some("Test Author".to_string()),
            published: Some(Utc::now()),
            guid: Some("test-guid".to_string()),
            categories: vec!["test".to_string()],
        };

        let article = Article::new(parsed_article, "test-feed");

        Feed {
            name: "test-feed".to_string(),
            url: "https://example.com/feed.xml".to_string(),
            title: Some("Test Feed".to_string()),
            description: Some("A test feed".to_string()),
            last_updated: Some(Utc::now()),
            articles: vec![article],
            status: FeedStatus::Active,
        }
    }

    #[test]
    fn test_filesystem_creation() {
        let fs = RssFuseFilesystem::new();
        
        // Root should exist
        let root = fs.inode_manager.get_node(FUSE_ROOT_ID).unwrap();
        assert_eq!(root.ino, FUSE_ROOT_ID);
        assert!(root.is_directory());
    }

    #[test]
    fn test_add_feed() {
        let fs = RssFuseFilesystem::new();
        let feed = create_test_feed();
        
        fs.add_feed(feed).unwrap();
        
        // Should have feed directory
        let feed_node = fs.inode_manager.get_node_by_name(FUSE_ROOT_ID, "test-feed").unwrap();
        assert!(feed_node.is_directory());
        
        // Should have article file
        let children = fs.inode_manager.list_children(feed_node.ino);
        assert_eq!(children.len(), 1);
        assert!(children[0].is_file());
    }

    #[test]
    fn test_remove_feed() {
        let fs = RssFuseFilesystem::new();
        let feed = create_test_feed();
        
        fs.add_feed(feed).unwrap();
        fs.remove_feed("test-feed").unwrap();
        
        // Feed directory should be gone
        assert!(fs.inode_manager.get_node_by_name(FUSE_ROOT_ID, "test-feed").is_none());
    }

    #[test]
    fn test_config_update() {
        let fs = RssFuseFilesystem::new();
        let config_content = r#"
[feeds]
"test-feed" = "https://example.com/feed.xml"

[settings]
refresh_interval = 300
"#.to_string();

        fs.update_config(config_content.clone());
        
        // Config content should be updated
        assert_eq!(*fs.config_content.read(), config_content);
    }

    #[test]
    fn test_meta_structure() {
        let fs = RssFuseFilesystem::new();
        
        // Should have .rss-fuse directory
        let meta = fs.inode_manager.get_node_by_name(FUSE_ROOT_ID, ".rss-fuse").unwrap();
        assert!(meta.is_directory());
        
        // Should have subdirectories and config file
        let children = fs.inode_manager.list_children(meta.ino);
        assert_eq!(children.len(), 3); // logs, cache, config.toml
        
        let names: Vec<String> = children.iter().map(|n| n.name.clone()).collect();
        assert!(names.contains(&"logs".to_string()));
        assert!(names.contains(&"cache".to_string()));
        assert!(names.contains(&"config.toml".to_string()));
    }

    #[test]
    fn test_node_to_file_attr() {
        let fs = RssFuseFilesystem::new();
        let root = fs.inode_manager.get_node(FUSE_ROOT_ID).unwrap();
        
        let attr = fs.node_to_file_attr(&root);
        assert_eq!(attr.ino, FUSE_ROOT_ID);
        assert_eq!(attr.kind, FileType::Directory);
        assert_eq!(attr.perm, 0o755);
    }
}