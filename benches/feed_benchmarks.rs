use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rss_fuse::feed::{parser::FeedParser, fetcher::FeedFetcher, Article};
use std::io::Cursor;
use std::time::Duration;
use tokio::runtime::Runtime;

mod test_data;
use test_data::*;

fn bench_parse_feeds(c: &mut Criterion) {
    let parser = FeedParser::new();
    
    let feeds = vec![
        ("simple_rss", SIMPLE_RSS),
        ("tech_news_rss", TECH_NEWS_RSS),
        ("science_atom", SCIENCE_BLOG_ATOM),
        ("unicode_rss", UNICODE_RSS),
        ("namespaced_rss", NAMESPACED_RSS),
        ("podcast_rss", PODCAST_RSS),
    ];
    
    let mut group = c.benchmark_group("feed_parsing");
    
    for (name, feed_content) in feeds {
        group.bench_with_input(
            BenchmarkId::new("parse", name),
            &feed_content,
            |b, content| {
                b.iter(|| {
                    let cursor = Cursor::new(content.as_bytes());
                    let result = parser.parse_feed(cursor);
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_large_feed_parsing(c: &mut Criterion) {
    let parser = FeedParser::new();
    
    // Create feeds with different numbers of articles
    let article_counts = vec![10, 100, 1000, 5000];
    
    let mut group = c.benchmark_group("large_feed_parsing");
    group.sample_size(10); // Fewer samples for large feeds
    group.measurement_time(Duration::from_secs(20));
    
    for &count in &article_counts {
        let large_feed = create_large_feed(count);
        
        group.bench_with_input(
            BenchmarkId::new("parse_articles", count),
            &large_feed,
            |b, feed_content| {
                b.iter(|| {
                    let cursor = Cursor::new(feed_content.as_bytes());
                    let result = parser.parse_feed(cursor);
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_article_creation(c: &mut Criterion) {
    let parser = FeedParser::new();
    let cursor = Cursor::new(TECH_NEWS_RSS.as_bytes());
    let parsed_feed = parser.parse_feed(cursor).unwrap();
    
    let mut group = c.benchmark_group("article_operations");
    
    // Benchmark article creation from parsed data
    group.bench_function("create_article", |b| {
        b.iter(|| {
            for parsed_article in &parsed_feed.articles {
                let article = Article::new(parsed_article.clone(), "test-feed");
                black_box(article);
            }
        });
    });
    
    // Benchmark text generation
    let articles: Vec<Article> = parsed_feed.articles.iter()
        .map(|parsed| Article::new(parsed.clone(), "test-feed"))
        .collect();
    
    group.bench_function("generate_text", |b| {
        b.iter(|| {
            for article in &articles {
                let text = article.to_text();
                black_box(text);
            }
        });
    });
    
    // Benchmark filename generation
    group.bench_function("generate_filename", |b| {
        b.iter(|| {
            for article in &articles {
                let filename = article.filename();
                black_box(filename);
            }
        });
    });
    
    group.finish();
}

fn bench_concurrent_parsing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let feeds = vec![
        TECH_NEWS_RSS,
        SCIENCE_BLOG_ATOM,
        NEWS_RSS,
        TECH_RSS,
        SCIENCE_RSS,
    ];
    
    let mut group = c.benchmark_group("concurrent_parsing");
    
    // Sequential parsing
    group.bench_function("sequential", |b| {
        b.to_async(&rt).iter(|| async {
            let parser = FeedParser::new();
            let mut results = Vec::new();
            
            for feed_content in &feeds {
                let cursor = Cursor::new(feed_content.as_bytes());
                let result = parser.parse_feed(cursor);
                results.push(black_box(result));
            }
            
            results
        });
    });
    
    // Concurrent parsing (simulated)
    group.bench_function("concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let parser = FeedParser::new();
            let tasks = feeds.iter().map(|feed_content| {
                let parser = parser.clone();
                let content = feed_content.to_string();
                tokio::spawn(async move {
                    let cursor = Cursor::new(content.as_bytes());
                    parser.parse_feed(cursor)
                })
            });
            
            let results = futures::future::join_all(tasks).await;
            black_box(results)
        });
    });
    
    group.finish();
}

fn bench_url_validation(c: &mut Criterion) {
    let parser = FeedParser::new();
    
    let urls = vec![
        "https://example.com/feed.xml",
        "http://example.com/rss",
        "https://subdomain.example.com/path/to/feed?param=value",
        "https://example.com/feed.xml#fragment",
        "https://user:pass@example.com/feed",
        "not-a-url",
        "ftp://example.com/feed",
        "file:///local/feed.xml",
        "",
    ];
    
    let mut group = c.benchmark_group("url_validation");
    
    for url in urls {
        group.bench_with_input(
            BenchmarkId::new("validate", url.len()),
            &url,
            |b, test_url| {
                b.iter(|| {
                    let result = parser.validate_feed_url(test_url);
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let parser = FeedParser::new();
    
    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10);
    
    // Test with increasingly large articles
    let content_sizes = vec![1024, 10240, 102400, 1024000]; // 1KB, 10KB, 100KB, 1MB
    
    for &size in &content_sizes {
        let large_content = "Lorem ipsum dolor sit amet. ".repeat(size / 28);
        let feed_with_large_content = create_feed_with_large_content(&large_content);
        
        group.bench_with_input(
            BenchmarkId::new("parse_large_content", size),
            &feed_with_large_content,
            |b, feed_content| {
                b.iter(|| {
                    let cursor = Cursor::new(feed_content.as_bytes());
                    let result = parser.parse_feed(cursor);
                    if let Ok(parsed) = result {
                        // Convert to Article to test full pipeline
                        let articles: Vec<Article> = parsed.articles.into_iter()
                            .map(|parsed_article| Article::new(parsed_article, "test"))
                            .collect();
                        black_box(articles);
                    }
                });
            },
        );
    }
    
    group.finish();
}

// Helper functions

fn create_large_feed(article_count: usize) -> String {
    let mut feed = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Large Feed</title>
        <description>A feed with many articles</description>
        <link>https://example.com</link>"#);
    
    for i in 0..article_count {
        feed.push_str(&format!(r#"
        <item>
            <title>Article Number {}</title>
            <link>https://example.com/article{}</link>
            <description>This is article number {} with some description content.</description>
            <pubDate>Wed, 15 Mar 2024 10:{}:00 GMT</pubDate>
            <guid>article-{}</guid>
            <category>category{}</category>
        </item>"#, i, i, i, i % 60, i, i % 10));
    }
    
    feed.push_str("\n    </channel>\n</rss>");
    feed
}

fn create_feed_with_large_content(content: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Large Content Feed</title>
        <description>Feed with large article content</description>
        <link>https://example.com</link>
        <item>
            <title>Large Article</title>
            <link>https://example.com/large</link>
            <description><![CDATA[{}]]></description>
            <pubDate>Wed, 15 Mar 2024 10:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#, content)
}

// Group all benchmarks
criterion_group!(
    benches,
    bench_parse_feeds,
    bench_large_feed_parsing,
    bench_article_creation,
    bench_concurrent_parsing,
    bench_url_validation,
    bench_memory_usage,
);

criterion_main!(benches);