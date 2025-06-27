pub mod extractor;

pub use extractor::{ContentExtractor, ArticleFrontmatter};

pub struct ContentSelectors {
    pub article: Vec<String>,
    pub content: Vec<String>,
    pub remove: Vec<String>,
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
            ],
        }
    }
}