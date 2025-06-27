use rss_fuse::feed::fetcher::FeedFetcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing feed module with real URL: https://commoncog.com/blog/rss/");
    
    let fetcher = FeedFetcher::new();
    let url = "https://commoncog.com/blog/rss/";
    
    println!("Fetching feed from: {}", url);
    
    match fetcher.fetch_feed(url).await {
        Ok(parsed_feed) => {
            println!("✓ Feed fetched and parsed successfully!");
            println!("Title: {}", parsed_feed.title);
            if let Some(description) = &parsed_feed.description {
                println!("Description: {}", description);
            }
            if let Some(link) = &parsed_feed.link {
                println!("Link: {}", link);
            }
            println!("Number of articles: {}", parsed_feed.articles.len());
            
            // Show first few articles
            for (i, article) in parsed_feed.articles.iter().take(3).enumerate() {
                println!("\nArticle {}:", i + 1);
                println!("  Title: {}", article.title);
                println!("  Link: {}", article.link);
                if let Some(description) = &article.description {
                    let truncated = if description.len() > 100 {
                        format!("{}...", &description[..100])
                    } else {
                        description.clone()
                    };
                    println!("  Description: {}", truncated);
                }
                if let Some(published) = &article.published {
                    println!("  Published: {}", published);
                }
                println!("  Categories: {:?}", article.categories);
            }
        }
        Err(e) => {
            println!("✗ Failed to fetch feed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}