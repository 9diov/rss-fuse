use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use fuser::FileType;
use crate::feed::Article;

/// Virtual filesystem node types
#[derive(Debug, Clone)]
pub enum NodeType {
    Root,
    FeedDirectory(String),  // Feed name
    ArticleFile(String, Arc<Article>),  // Feed name, Article data
    MetaDirectory,  // .rss-fuse directory for metadata
    ConfigFile,     // config.toml
    LogsDirectory,  // logs directory
    CacheDirectory, // cache directory
}

/// Virtual filesystem node
#[derive(Debug, Clone)]
pub struct VNode {
    pub ino: u64,
    pub parent_ino: u64,
    pub name: String,
    pub node_type: NodeType,
    pub file_type: FileType,
    pub size: u64,
    pub children: Vec<u64>, // Child inode numbers
}

impl VNode {
    pub fn new(ino: u64, parent_ino: u64, name: String, node_type: NodeType) -> Self {
        let (file_type, size) = match &node_type {
            NodeType::Root | 
            NodeType::FeedDirectory(_) | 
            NodeType::MetaDirectory | 
            NodeType::LogsDirectory | 
            NodeType::CacheDirectory => (FileType::Directory, 0),
            NodeType::ArticleFile(feed_name, article) => {
                // Use markdown format by default, fallback to text on error
                let content = article.to_markdown(feed_name)
                    .unwrap_or_else(|_| article.to_text());
                (FileType::RegularFile, content.len() as u64)
            },
            NodeType::ConfigFile => (FileType::RegularFile, 0), // Will be computed when needed
        };

        Self {
            ino,
            parent_ino,
            name,
            node_type,
            file_type,
            size,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child_ino: u64) {
        if !self.children.contains(&child_ino) {
            self.children.push(child_ino);
        }
    }

    pub fn remove_child(&mut self, child_ino: u64) {
        self.children.retain(|&ino| ino != child_ino);
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.file_type, FileType::Directory)
    }

    pub fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::RegularFile)
    }
}

/// Inode manager for the virtual filesystem
pub struct InodeManager {
    nodes: RwLock<HashMap<u64, VNode>>,
    next_ino: RwLock<u64>,
    name_to_ino: RwLock<HashMap<(u64, String), u64>>, // (parent_ino, name) -> ino
}

impl InodeManager {
    pub fn new() -> Self {
        let mut manager = Self {
            nodes: RwLock::new(HashMap::new()),
            next_ino: RwLock::new(2), // Start from 2, 1 is reserved for root
            name_to_ino: RwLock::new(HashMap::new()),
        };

        // Create root directory
        manager.create_root();
        manager
    }

    fn create_root(&self) {
        let root = VNode::new(1, 1, "/".to_string(), NodeType::Root);
        self.nodes.write().insert(1, root);
        self.name_to_ino.write().insert((1, "/".to_string()), 1);
    }

    pub fn allocate_ino(&self) -> u64 {
        let mut next_ino = self.next_ino.write();
        let ino = *next_ino;
        *next_ino += 1;
        ino
    }

    pub fn get_node(&self, ino: u64) -> Option<VNode> {
        self.nodes.read().get(&ino).cloned()
    }

    pub fn get_node_by_name(&self, parent_ino: u64, name: &str) -> Option<VNode> {
        let name_to_ino = self.name_to_ino.read();
        let ino = name_to_ino.get(&(parent_ino, name.to_string()))?;
        self.nodes.read().get(ino).cloned()
    }

    pub fn create_node(&self, parent_ino: u64, name: String, node_type: NodeType) -> Result<u64, String> {
        // Check if parent exists and is a directory
        let parent = self.get_node(parent_ino)
            .ok_or("Parent directory not found")?;
        
        if !parent.is_directory() {
            return Err("Parent is not a directory".to_string());
        }

        // Check if name already exists
        if self.get_node_by_name(parent_ino, &name).is_some() {
            return Err("File already exists".to_string());
        }

        let ino = self.allocate_ino();
        let node = VNode::new(ino, parent_ino, name.clone(), node_type);

        // Add to parent's children
        {
            let mut nodes = self.nodes.write();
            nodes.insert(ino, node);
            if let Some(parent) = nodes.get_mut(&parent_ino) {
                parent.add_child(ino);
            }
        }

        // Add to name lookup
        self.name_to_ino.write().insert((parent_ino, name), ino);

        Ok(ino)
    }

    pub fn remove_node(&self, ino: u64) -> Result<(), String> {
        if ino == 1 {
            return Err("Cannot remove root directory".to_string());
        }

        let node = self.get_node(ino)
            .ok_or("Node not found")?;

        // Remove from parent's children
        {
            let mut nodes = self.nodes.write();
            if let Some(parent) = nodes.get_mut(&node.parent_ino) {
                parent.remove_child(ino);
            }
            nodes.remove(&ino);
        }

        // Remove from name lookup
        self.name_to_ino.write().remove(&(node.parent_ino, node.name));

        Ok(())
    }

    pub fn list_children(&self, parent_ino: u64) -> Vec<VNode> {
        let nodes = self.nodes.read();
        if let Some(parent) = nodes.get(&parent_ino) {
            parent.children.iter()
                .filter_map(|&child_ino| nodes.get(&child_ino))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn update_node_size(&self, ino: u64, size: u64) {
        if let Some(node) = self.nodes.write().get_mut(&ino) {
            node.size = size;
        }
    }

    pub fn create_feed_directory(&self, feed_name: &str) -> Result<u64, String> {
        self.create_node(1, feed_name.to_string(), NodeType::FeedDirectory(feed_name.to_string()))
    }

    pub fn create_article_file(&self, feed_name: &str, article: Arc<Article>) -> Result<u64, String> {
        // Get or create feed directory
        let feed_ino = match self.get_node_by_name(1, feed_name) {
            Some(node) => node.ino,
            None => self.create_feed_directory(feed_name)?,
        };

        let filename = article.markdown_filename();
        self.create_node(feed_ino, filename, NodeType::ArticleFile(feed_name.to_string(), article))
    }

    pub fn create_meta_structure(&self) -> Result<(), String> {
        // Create .rss-fuse directory
        let meta_ino = self.create_node(1, ".rss-fuse".to_string(), NodeType::MetaDirectory)?;
        
        // Create subdirectories
        self.create_node(meta_ino, "logs".to_string(), NodeType::LogsDirectory)?;
        self.create_node(meta_ino, "cache".to_string(), NodeType::CacheDirectory)?;
        
        // Create config file
        self.create_node(meta_ino, "config.toml".to_string(), NodeType::ConfigFile)?;
        
        Ok(())
    }

    pub fn get_total_nodes(&self) -> usize {
        self.nodes.read().len()
    }

    pub fn get_article_content(&self, ino: u64) -> Option<String> {
        let nodes = self.nodes.read();
        if let Some(node) = nodes.get(&ino) {
            match &node.node_type {
                NodeType::ArticleFile(feed_name, article) => {
                    // Use markdown format by default, fallback to text on error
                    Some(article.to_markdown(feed_name)
                        .unwrap_or_else(|_| article.to_text()))
                },
                _ => None,
            }
        } else {
            None
        }
    }
}

impl Default for InodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{Article, ParsedArticle};
    use chrono::Utc;

    fn create_test_article() -> Article {
        let parsed = ParsedArticle {
            title: "Test Article".to_string(),
            link: "https://example.com/test".to_string(),
            description: Some("Test description".to_string()),
            content: None,
            author: Some("Test Author".to_string()),
            published: Some(Utc::now()),
            guid: Some("test-guid".to_string()),
            categories: vec!["test".to_string()],
        };
        Article::new(parsed, "test-feed")
    }

    #[test]
    fn test_inode_manager_creation() {
        let manager = InodeManager::new();
        
        // Root should exist
        let root = manager.get_node(1).unwrap();
        assert_eq!(root.ino, 1);
        assert_eq!(root.name, "/");
        assert!(root.is_directory());
    }

    #[test]
    fn test_create_feed_directory() {
        let manager = InodeManager::new();
        
        let feed_ino = manager.create_feed_directory("tech-news").unwrap();
        let feed_node = manager.get_node(feed_ino).unwrap();
        
        assert_eq!(feed_node.name, "tech-news");
        assert_eq!(feed_node.parent_ino, 1);
        assert!(feed_node.is_directory());
        
        // Should be able to find by name
        let found = manager.get_node_by_name(1, "tech-news").unwrap();
        assert_eq!(found.ino, feed_ino);
    }

    #[test]
    fn test_create_article_file() {
        let manager = InodeManager::new();
        let article = Arc::new(create_test_article());
        
        let article_ino = manager.create_article_file("tech-news", article.clone()).unwrap();
        let article_node = manager.get_node(article_ino).unwrap();
        
        assert_eq!(article_node.name, article.filename());
        assert!(article_node.is_file());
        assert!(article_node.size > 0);
        
        // Content should be retrievable
        let content = manager.get_article_content(article_ino).unwrap();
        assert!(content.contains("Test Article"));
    }

    #[test]
    fn test_meta_structure_creation() {
        let manager = InodeManager::new();
        
        manager.create_meta_structure().unwrap();
        
        // Should have .rss-fuse directory
        let meta = manager.get_node_by_name(1, ".rss-fuse").unwrap();
        assert!(meta.is_directory());
        
        // Should have subdirectories
        let logs = manager.get_node_by_name(meta.ino, "logs").unwrap();
        assert!(logs.is_directory());
        
        let cache = manager.get_node_by_name(meta.ino, "cache").unwrap();
        assert!(cache.is_directory());
        
        let config = manager.get_node_by_name(meta.ino, "config.toml").unwrap();
        assert!(config.is_file());
    }

    #[test]
    fn test_directory_listing() {
        let manager = InodeManager::new();
        let article = Arc::new(create_test_article());
        
        manager.create_article_file("tech-news", article).unwrap();
        manager.create_meta_structure().unwrap();
        
        // List root directory
        let children = manager.list_children(1);
        assert_eq!(children.len(), 2); // tech-news and .rss-fuse
        
        let names: Vec<String> = children.iter().map(|n| n.name.clone()).collect();
        assert!(names.contains(&"tech-news".to_string()));
        assert!(names.contains(&".rss-fuse".to_string()));
    }

    #[test]
    fn test_duplicate_prevention() {
        let manager = InodeManager::new();
        
        // Create first feed
        manager.create_feed_directory("tech-news").unwrap();
        
        // Try to create duplicate
        let result = manager.create_feed_directory("tech-news");
        assert!(result.is_err());
    }

    #[test]
    fn test_node_removal() {
        let manager = InodeManager::new();
        
        let feed_ino = manager.create_feed_directory("tech-news").unwrap();
        assert!(manager.get_node(feed_ino).is_some());
        
        manager.remove_node(feed_ino).unwrap();
        assert!(manager.get_node(feed_ino).is_none());
        
        // Should not be in parent's children
        let root_children = manager.list_children(1);
        assert!(root_children.iter().all(|n| n.ino != feed_ino));
    }
}