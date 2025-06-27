/// Test data for benchmarking feed parsing performance

pub const SIMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Simple Feed</title>
        <description>A minimal RSS feed</description>
        <link>https://simple.example.com</link>
        <item>
            <title>Simple Article</title>
            <link>https://simple.example.com/article</link>
            <description>A simple article for testing.</description>
        </item>
    </channel>
</rss>"#;

pub const TECH_NEWS_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Tech News Daily</title>
        <description>Latest technology news and updates</description>
        <link>https://technews.example.com</link>
        <item>
            <title>AI Revolution in 2024</title>
            <link>https://technews.example.com/ai-revolution-2024</link>
            <description>The artificial intelligence landscape is rapidly evolving.</description>
            <author>editor@technews.example.com (John Doe)</author>
            <category>AI</category>
            <category>Technology</category>
            <pubDate>Thu, 16 Mar 2024 10:00:00 GMT</pubDate>
        </item>
        <item>
            <title>Quantum Computing Breakthrough</title>
            <link>https://technews.example.com/quantum-breakthrough</link>
            <description>Scientists have achieved a new milestone in quantum computing.</description>
            <category>Quantum</category>
            <pubDate>Thu, 16 Mar 2024 08:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#;

pub const SCIENCE_BLOG_ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Science Discoveries</title>
    <subtitle>Exploring the frontiers of scientific knowledge</subtitle>
    <link href="https://science.example.com"/>
    <updated>2024-03-16T12:00:00Z</updated>
    <id>https://science.example.com/</id>
    <entry>
        <title>Quantum Computing Advances</title>
        <link href="https://science.example.com/quantum-advances"/>
        <id>https://science.example.com/quantum-advances</id>
        <updated>2024-03-16T10:00:00Z</updated>
        <summary>Recent breakthroughs in quantum computing technology</summary>
        <content type="html">The field of quantum computing has seen remarkable progress.</content>
    </entry>
</feed>"#;

pub const NEWS_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Daily News</title>
        <description>Breaking news and current events</description>
        <link>https://news.example.com</link>
        <item>
            <title>Breaking: Major Economic Update</title>
            <link>https://news.example.com/economic-update</link>
            <description>Economic indicators show positive trends.</description>
        </item>
    </channel>
</rss>"#;

pub const TECH_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Tech Updates</title>
        <description>Latest in technology</description>
        <link>https://tech.example.com</link>
        <item>
            <title>New Programming Language Released</title>
            <link>https://tech.example.com/new-language</link>
            <description>Developers excited about new language features.</description>
        </item>
    </channel>
</rss>"#;

pub const SCIENCE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Science Today</title>
        <description>Scientific discoveries and research</description>
        <link>https://science-today.example.com</link>
        <item>
            <title>Climate Change Research Findings</title>
            <link>https://science-today.example.com/climate-research</link>
            <description>New research sheds light on climate patterns.</description>
        </item>
    </channel>
</rss>"#;

pub const UNICODE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Café Internacional</title>
        <description>Noticias internacionales</description>
        <link>https://cafe.example.com</link>
        <item>
            <title>Naïve résumé of économic situation</title>
            <link>https://cafe.example.com/economia</link>
            <description>Análisis de la situación económica actual</description>
        </item>
    </channel>
</rss>"#;

pub const NAMESPACED_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
    <channel>
        <title>Namespaced Feed</title>
        <description>Feed with custom namespaces</description>
        <link>https://namespaced.example.com</link>
        <item>
            <title>Article with Custom Elements</title>
            <link>https://namespaced.example.com/article</link>
            <description>Basic description</description>
            <content:encoded>Full content with HTML formatting.</content:encoded>
        </item>
    </channel>
</rss>"#;

pub const PODCAST_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Tech Podcast</title>
        <description>Weekly technology discussions</description>
        <link>https://podcast.example.com</link>
        <item>
            <title>Episode 1: AI in Daily Life</title>
            <link>https://podcast.example.com/episode1</link>
            <description>Discussion about AI integration.</description>
            <enclosure url="https://podcast.example.com/episode1.mp3" length="52428800" type="audio/mpeg"/>
            <itunes:duration>00:45:30</itunes:duration>
        </item>
    </channel>
</rss>"#;