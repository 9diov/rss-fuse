use rss_fuse::content::ContentExtractor;
use rss_fuse::feed::{Article, ParsedArticle};
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("RSS-FUSE Markdown Content Extraction Demo");
    println!("=========================================\n");

    // Create a sample article with HTML content
    let parsed_article = ParsedArticle {
        title: "Rust 1.75.0 Released".to_string(),
        link: "https://blog.rust-lang.org/2023/12/28/Rust-1.75.0.html".to_string(),
        description: Some("The Rust team is pleased to announce the latest release.".to_string()),
        content: Some(r#"
            <h1>Rust 1.75.0 Released</h1>
            <p>The Rust team is pleased to announce a new version of Rust, <strong>1.75.0</strong>.</p>
            <h2>What's in 1.75.0 stable</h2>
            <p>This release includes several new features:</p>
            <ul>
                <li><code>async fn</code> in traits</li>
                <li>Return position <em>impl Trait</em> in traits</li>
                <li>Associated types with bounds in <code>where</code> clauses</li>
            </ul>
            <h3>Example Code</h3>
            <pre><code class="language-rust">
async fn example() -> impl Future&lt;Output = ()&gt; {
    async {
        println!("Hello from async Rust!");
    }
}
            </code></pre>
            <blockquote>
                <p>This is a significant milestone for async programming in Rust.</p>
            </blockquote>
            <p>For more information, visit the <a href="https://blog.rust-lang.org">official blog</a>.</p>
        "#.to_string()),
        author: Some("The Rust Team".to_string()),
        published: Some(Utc::now()),
        guid: Some("rust-1-75-0".to_string()),
        categories: vec!["rust".to_string(), "programming".to_string(), "release".to_string()],
    };

    // Convert to Article
    let article = Article::new(parsed_article, "rust-blog");

    // Create content extractor
    let extractor = ContentExtractor::new()?;

    // Extract content as Markdown with YAML frontmatter
    let markdown_content = extractor.extract_article(&article, "rust-blog")?;

    println!("Generated Markdown (.md file content):");
    println!("======================================\n");
    println!("{}", markdown_content);

    println!("\n");
    println!("Filename: {}", article.markdown_filename());

    // Demonstrate the difference with legacy text format
    println!("\n");
    println!("Legacy Text Format (.txt file content):");
    println!("=======================================\n");
    println!("{}", article.to_text());

    Ok(())
}