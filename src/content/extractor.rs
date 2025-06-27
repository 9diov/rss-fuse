use crate::error::{Error, Result};
use crate::feed::{Article, ParsedArticle};
use chrono::{DateTime, Utc};
use html2md::parse_html;
use regex::Regex;
use select::document::Document;
use serde::{Deserialize, Serialize};

/// Content extractor for converting HTML articles to Markdown with YAML frontmatter
pub struct ContentExtractor {
    selectors: ContentSelectors,
    regex_patterns: RegexPatterns,
}

#[derive(Debug, Clone)]
pub struct ContentSelectors {
    pub article: Vec<String>,
    pub content: Vec<String>,
    pub remove: Vec<String>,
}

#[derive(Debug)]
struct RegexPatterns {
    code_block: Regex,
    inline_code: Regex,
    whitespace: Regex,
    multiple_newlines: Regex,
}

/// YAML frontmatter structure for articles
#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleFrontmatter {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<DateTime<Utc>>,
    pub url: String,
    pub feed: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guid: Option<String>,
}

impl Default for ContentSelectors {
    fn default() -> Self {
        Self {
            article: vec![
                "article".to_string(),
                ".post-content".to_string(),
                ".entry-content".to_string(),
                ".content".to_string(),
                "main".to_string(),
            ],
            content: vec![
                "p".to_string(),
                "h1, h2, h3, h4, h5, h6".to_string(),
                "blockquote".to_string(),
                "pre".to_string(),
                "code".to_string(),
                "ul, ol, li".to_string(),
                "img".to_string(),
                "a".to_string(),
            ],
            remove: vec![
                ".advertisement".to_string(),
                ".ads".to_string(),
                ".social-share".to_string(),
                ".comments".to_string(),
                "script".to_string(),
                "style".to_string(),
                "nav".to_string(),
                "footer".to_string(),
                "header".to_string(),
                ".sidebar".to_string(),
                ".related-posts".to_string(),
            ],
        }
    }
}

impl RegexPatterns {
    fn new() -> Result<Self> {
        Ok(Self {
            code_block: Regex::new(r"```[\s\S]*?```").map_err(|e| Error::ContentExtraction(e.to_string()))?,
            inline_code: Regex::new(r"`[^`]+`").map_err(|e| Error::ContentExtraction(e.to_string()))?,
            whitespace: Regex::new(r"\s+").map_err(|e| Error::ContentExtraction(e.to_string()))?,
            multiple_newlines: Regex::new(r"\n{3,}").map_err(|e| Error::ContentExtraction(e.to_string()))?,
        })
    }
}

impl ContentExtractor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            selectors: ContentSelectors::default(),
            regex_patterns: RegexPatterns::new()?,
        })
    }

    pub fn with_selectors(selectors: ContentSelectors) -> Result<Self> {
        Ok(Self {
            selectors,
            regex_patterns: RegexPatterns::new()?,
        })
    }

    /// Extract and convert article content to Markdown format with YAML frontmatter
    pub fn extract_article(&self, article: &Article, feed_name: &str) -> Result<String> {
        let frontmatter = self.create_frontmatter(article, feed_name)?;
        let content = self.extract_content(article)?;
        
        let yaml_frontmatter = serde_yaml::to_string(&frontmatter)
            .map_err(|e| Error::ContentExtraction(format!("Failed to serialize YAML frontmatter: {}", e)))?;
        
        Ok(format!("---\n{}---\n\n{}", yaml_frontmatter, content))
    }

    /// Create YAML frontmatter from article metadata
    fn create_frontmatter(&self, article: &Article, feed_name: &str) -> Result<ArticleFrontmatter> {
        Ok(ArticleFrontmatter {
            title: article.title.clone(),
            author: article.author.clone(),
            date: article.published,
            url: article.link.clone(),
            feed: feed_name.to_string(),
            tags: article.tags.clone(),
            categories: vec![], // Could be extracted from content or feed metadata
            description: article.description.clone(),
            guid: Some(article.id.clone()),
        })
    }

    /// Extract and convert content to Markdown
    fn extract_content(&self, article: &Article) -> Result<String> {
        let html_content = article.content
            .as_ref()
            .or(article.description.as_ref())
            .ok_or_else(|| Error::ContentExtraction("No content available".to_string()))?;

        // Clean HTML first
        let cleaned_html = self.clean_html(html_content)?;
        
        // Convert to Markdown
        let markdown = self.html_to_markdown(&cleaned_html)?;
        
        // Post-process the Markdown
        let processed_markdown = self.post_process_markdown(markdown)?;
        
        Ok(processed_markdown)
    }

    /// Clean HTML content by removing unwanted elements
    fn clean_html(&self, html: &str) -> Result<String> {
        let document = Document::from(html);
        let mut cleaned_html = html.to_string();

        // Remove unwanted elements
        for selector in &self.selectors.remove {
            // This is a simplified approach - in a real implementation, 
            // we'd need more sophisticated HTML manipulation
            if selector.starts_with('.') {
                let class_name = &selector[1..];
                cleaned_html = cleaned_html.replace(&format!("<div class=\"{}\">", class_name), "");
                cleaned_html = cleaned_html.replace(&format!("<span class=\"{}\">", class_name), "");
            } else if selector.starts_with('#') {
                let id_name = &selector[1..];
                cleaned_html = cleaned_html.replace(&format!("<div id=\"{}\">", id_name), "");
            } else {
                // Remove by tag name
                let tag_regex = Regex::new(&format!(r"<{}[^>]*>.*?</{}>", selector, selector))
                    .map_err(|e| Error::ContentExtraction(e.to_string()))?;
                cleaned_html = tag_regex.replace_all(&cleaned_html, "").to_string();
            }
        }

        Ok(cleaned_html)
    }

    /// Convert HTML to Markdown using html2md
    fn html_to_markdown(&self, html: &str) -> Result<String> {
        // Use html2md for basic conversion
        let markdown = parse_html(html);
        Ok(markdown)
    }

    /// Post-process Markdown for better formatting
    fn post_process_markdown(&self, markdown: String) -> Result<String> {
        let mut processed = markdown;

        // Clean up excessive whitespace
        processed = self.regex_patterns.whitespace
            .replace_all(&processed, " ")
            .to_string();

        // Limit consecutive newlines to maximum of 2
        processed = self.regex_patterns.multiple_newlines
            .replace_all(&processed, "\n\n")
            .to_string();

        // Add title as H1 if not present
        if !processed.starts_with('#') {
            processed = format!("# Article Content\n\n{}", processed);
        }

        // Ensure proper spacing around code blocks
        processed = processed.replace("```\n\n", "```\n");
        processed = processed.replace("\n\n```", "\n```");

        // Trim and ensure single trailing newline
        processed = processed.trim().to_string();
        if !processed.ends_with('\n') {
            processed.push('\n');
        }

        Ok(processed)
    }

    /// Extract content from ParsedArticle (for use during feed parsing)
    pub fn extract_parsed_article(&self, parsed: &ParsedArticle, feed_name: &str) -> Result<String> {
        let temp_article = Article {
            id: parsed.guid.clone().unwrap_or_else(|| "temp".to_string()),
            title: parsed.title.clone(),
            link: parsed.link.clone(),
            description: parsed.description.clone(),
            content: parsed.content.clone(),
            author: parsed.author.clone(),
            published: parsed.published,
            updated: None,
            tags: parsed.categories.clone(),
            read: false,
            cached_at: Some(Utc::now()),
        };

        self.extract_article(&temp_article, feed_name)
    }

    /// Extract categories from content or metadata
    pub fn extract_categories(&self, article: &Article) -> Vec<String> {
        let mut categories = Vec::new();
        
        // Add existing tags as categories
        categories.extend_from_slice(&article.tags);
        
        // Extract categories from content using simple keyword matching
        if let Some(content) = &article.content {
            let content_lower = content.to_lowercase();
            
            // Common technology categories
            let tech_keywords = [
                ("rust", "Programming"),
                ("javascript", "Programming"),
                ("python", "Programming"),
                ("react", "Web Development"),
                ("docker", "DevOps"),
                ("kubernetes", "DevOps"),
                ("ai", "Artificial Intelligence"),
                ("machine learning", "Artificial Intelligence"),
                ("blockchain", "Technology"),
                ("security", "Security"),
                ("startup", "Business"),
                ("design", "Design"),
            ];
            
            for (keyword, category) in &tech_keywords {
                if content_lower.contains(keyword) && !categories.contains(&category.to_string()) {
                    categories.push(category.to_string());
                }
            }
        }
        
        // Remove duplicates and limit to reasonable number
        categories.sort();
        categories.dedup();
        categories.truncate(5);
        
        categories
    }
}

impl Default for ContentExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create default ContentExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_article() -> Article {
        Article {
            id: "test-123".to_string(),
            title: "Test Article".to_string(),
            link: "https://example.com/test".to_string(),
            description: Some("A test description".to_string()),
            content: Some("<h1>Test Content</h1><p>This is a <strong>test</strong> article with <a href=\"https://example.com\">a link</a>.</p><pre><code>fn main() { println!(\"Hello!\"); }</code></pre>".to_string()),
            author: Some("Test Author".to_string()),
            published: Some(Utc::now()),
            updated: None,
            tags: vec!["rust".to_string(), "programming".to_string()],
            read: false,
            cached_at: Some(Utc::now()),
        }
    }

    #[test]
    fn test_content_extractor_creation() {
        let extractor = ContentExtractor::new();
        assert!(extractor.is_ok());
    }

    #[test]
    fn test_frontmatter_creation() {
        let extractor = ContentExtractor::new().unwrap();
        let article = create_test_article();
        
        let frontmatter = extractor.create_frontmatter(&article, "test-feed").unwrap();
        
        assert_eq!(frontmatter.title, "Test Article");
        assert_eq!(frontmatter.feed, "test-feed");
        assert_eq!(frontmatter.url, "https://example.com/test");
        assert_eq!(frontmatter.tags, vec!["rust", "programming"]);
    }

    #[test]
    fn test_html_to_markdown_conversion() {
        let extractor = ContentExtractor::new().unwrap();
        let html = "<h1>Title</h1><p>This is <strong>bold</strong> text.</p>";
        
        let markdown = extractor.html_to_markdown(html).unwrap();
        
        // html2md uses underline style for H1 and **bold** for strong tags
        assert!(markdown.contains("Title"));
        assert!(markdown.contains("**bold**"));
    }

    #[test]
    fn test_extract_article() {
        let extractor = ContentExtractor::new().unwrap();
        let article = create_test_article();
        
        let result = extractor.extract_article(&article, "test-feed").unwrap();
        
        assert!(result.starts_with("---"));
        assert!(result.contains("title: Test Article"));
        assert!(result.contains("feed: test-feed"));
        assert!(result.contains("---\n\n"));
        assert!(result.contains("# Article Content"));
    }

    #[test]
    fn test_category_extraction() {
        let extractor = ContentExtractor::new().unwrap();
        let mut article = create_test_article();
        article.content = Some("This article discusses Rust programming and Docker containers.".to_string());
        
        let categories = extractor.extract_categories(&article);
        
        assert!(categories.contains(&"Programming".to_string()));
        assert!(categories.contains(&"DevOps".to_string()));
    }

    #[test]
    fn test_clean_html() {
        let extractor = ContentExtractor::new().unwrap();
        let html = "<p>Content</p><script>alert('test');</script><div class=\"ads\">Ad content</div>";
        
        let cleaned = extractor.clean_html(html).unwrap();
        
        assert!(!cleaned.contains("script"));
        assert!(cleaned.contains("Content"));
    }
}