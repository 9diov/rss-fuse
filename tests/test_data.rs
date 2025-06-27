/// Test data for feed parsing and fetching tests
/// Contains various RSS and Atom feed samples for comprehensive testing

pub const TECH_NEWS_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Tech News Daily</title>
        <description>Latest technology news and updates</description>
        <link>https://technews.example.com</link>
        <language>en-us</language>
        <lastBuildDate>Thu, 16 Mar 2024 12:00:00 GMT</lastBuildDate>
        <webMaster>editor@technews.example.com</webMaster>
        <managingEditor>editor@technews.example.com</managingEditor>
        
        <item>
            <title>AI Revolution in 2024</title>
            <link>https://technews.example.com/ai-revolution-2024</link>
            <description>The artificial intelligence landscape is rapidly evolving with new breakthroughs.</description>
            <author>editor@technews.example.com (John Doe)</author>
            <category>AI</category>
            <category>Technology</category>
            <pubDate>Thu, 16 Mar 2024 10:00:00 GMT</pubDate>
            <guid isPermaLink="true">https://technews.example.com/ai-revolution-2024</guid>
        </item>
        
        <item>
            <title>Quantum Computing Breakthrough</title>
            <link>https://technews.example.com/quantum-breakthrough</link>
            <description><![CDATA[Scientists have achieved a new milestone in <strong>quantum computing</strong> research.]]></description>
            <author>editor@technews.example.com (Jane Smith)</author>
            <category>Quantum</category>
            <category>Research</category>
            <pubDate>Thu, 16 Mar 2024 08:00:00 GMT</pubDate>
            <guid>quantum-breakthrough-2024-03-16</guid>
        </item>
        
        <item>
            <title>Cybersecurity Trends</title>
            <link>https://technews.example.com/cybersecurity-trends</link>
            <description>New cybersecurity threats and defense strategies for 2024.</description>
            <author>editor@technews.example.com (Bob Wilson)</author>
            <category>Security</category>
            <category>Trends</category>
            <pubDate>Thu, 16 Mar 2024 06:00:00 GMT</pubDate>
            <guid>cybersecurity-trends-2024</guid>
            <enclosure url="https://technews.example.com/audio/cybersecurity.mp3" length="12345678" type="audio/mpeg"/>
        </item>
    </channel>
</rss>"#;

pub const SCIENCE_BLOG_ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Science Discoveries</title>
    <subtitle>Exploring the frontiers of scientific knowledge</subtitle>
    <link href="https://science.example.com"/>
    <link rel="self" href="https://science.example.com/feed.xml"/>
    <updated>2024-03-16T12:00:00Z</updated>
    <id>https://science.example.com/</id>
    <author>
        <name>Science Team</name>
        <email>team@science.example.com</email>
    </author>
    
    <entry>
        <title>Quantum Computing Advances</title>
        <link href="https://science.example.com/quantum-advances"/>
        <id>https://science.example.com/quantum-advances</id>
        <updated>2024-03-16T10:00:00Z</updated>
        <published>2024-03-16T10:00:00Z</published>
        <author>
            <name>Dr. Alice Johnson</name>
            <email>alice@science.example.com</email>
        </author>
        <category term="quantum computing"/>
        <category term="physics"/>
        <summary>Recent breakthroughs in quantum computing technology</summary>
        <content type="html"><![CDATA[
            <p>The field of <strong>quantum computing</strong> has seen remarkable progress this year.</p>
            <p>Key developments include:</p>
            <ul>
                <li>Improved quantum error correction</li>
                <li>New quantum algorithms</li>
                <li>Better qubit stability</li>
            </ul>
            <p>These advances bring us closer to practical quantum applications.</p>
        ]]></content>
    </entry>
    
    <entry>
        <title>Mars Exploration Update</title>
        <link href="https://science.example.com/mars-exploration"/>
        <id>https://science.example.com/mars-exploration</id>
        <updated>2024-03-15T14:00:00Z</updated>
        <published>2024-03-15T14:00:00Z</published>
        <author>
            <name>Dr. Bob Martinez</name>
            <email>bob@science.example.com</email>
        </author>
        <category term="space"/>
        <category term="mars"/>
        <summary>Latest findings from Mars rovers and orbiters</summary>
        <content type="html"><![CDATA[
            <p>Mars exploration missions continue to provide fascinating insights.</p>
            <img src="https://science.example.com/images/mars-surface.jpg" alt="Mars surface"/>
            <p>Recent discoveries include evidence of ancient water flows and potential organic compounds.</p>
        ]]></content>
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
            <description>Economic indicators show positive trends this quarter.</description>
            <pubDate>Thu, 16 Mar 2024 11:00:00 GMT</pubDate>
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
            <pubDate>Thu, 16 Mar 2024 09:00:00 GMT</pubDate>
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
            <pubDate>Thu, 16 Mar 2024 07:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#;

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

pub const MALFORMED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Broken Feed</title>
        <description>This feed has malformed XML</description>
        <item>
            <title>Broken Article</title>
            <link>https://broken.example.com/article
            <description>Missing closing tags</description>
        </item>
    </channel>
    <!-- Missing closing rss tag -->"#;

pub const UNICODE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Café Internacional</title>
        <description>Noticias internacionales en español</description>
        <link>https://cafe.example.com</link>
        <item>
            <title>Naïve résumé of économic situation</title>
            <link>https://cafe.example.com/economia</link>
            <description>Análisis de la situación económica actual con caracteres especiales: àáâãäåæçèéêë</description>
            <pubDate>Thu, 16 Mar 2024 15:00:00 GMT</pubDate>
        </item>
        <item>
            <title>文章标题中文测试</title>
            <link>https://cafe.example.com/chinese</link>
            <description>这是一个中文描述，测试Unicode字符处理能力。</description>
            <pubDate>Thu, 16 Mar 2024 14:00:00 GMT</pubDate>
        </item>
        <item>
            <title>Тест кириллицы</title>
            <link>https://cafe.example.com/cyrillic</link>
            <description>Проверка поддержки кириллических символов в RSS-ленте.</description>
            <pubDate>Thu, 16 Mar 2024 13:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#;

pub const NAMESPACED_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" 
     xmlns:content="http://purl.org/rss/1.0/modules/content/"
     xmlns:dc="http://purl.org/dc/elements/1.1/"
     xmlns:atom="http://www.w3.org/2005/Atom"
     xmlns:media="http://search.yahoo.com/mrss/">
    <channel>
        <title>Namespaced Feed</title>
        <description>Feed with custom namespaces</description>
        <link>https://namespaced.example.com</link>
        <atom:link href="https://namespaced.example.com/feed.xml" rel="self" type="application/rss+xml"/>
        
        <item>
            <title>Article with Custom Elements</title>
            <link>https://namespaced.example.com/article</link>
            <description>Basic description</description>
            <content:encoded><![CDATA[
                <p>This is the full content with <strong>HTML</strong> formatting.</p>
                <p>It includes rich media and detailed information.</p>
            ]]></content:encoded>
            <dc:creator>Content Author</dc:creator>
            <dc:date>2024-03-16T12:00:00Z</dc:date>
            <media:content url="https://namespaced.example.com/image.jpg" type="image/jpeg"/>
            <media:description>Sample image description</media:description>
            <pubDate>Thu, 16 Mar 2024 12:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#;

pub const PODCAST_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Tech Podcast</title>
        <description>Weekly technology discussions</description>
        <link>https://podcast.example.com</link>
        <language>en-us</language>
        <itunes:category text="Technology"/>
        <itunes:image href="https://podcast.example.com/artwork.jpg"/>
        
        <item>
            <title>Episode 1: AI in Daily Life</title>
            <link>https://podcast.example.com/episode1</link>
            <description>Discussion about AI integration in everyday applications.</description>
            <enclosure url="https://podcast.example.com/episode1.mp3" length="52428800" type="audio/mpeg"/>
            <pubDate>Thu, 16 Mar 2024 18:00:00 GMT</pubDate>
            <itunes:duration>00:45:30</itunes:duration>
            <itunes:explicit>no</itunes:explicit>
        </item>
        
        <item>
            <title>Episode 2: Quantum Computing Explained</title>
            <link>https://podcast.example.com/episode2</link>
            <description>Breaking down quantum computing concepts for general audiences.</description>
            <enclosure url="https://podcast.example.com/episode2.mp3" length="48234567" type="audio/mpeg"/>
            <pubDate>Thu, 09 Mar 2024 18:00:00 GMT</pubDate>
            <itunes:duration>00:42:15</itunes:duration>
            <itunes:explicit>no</itunes:explicit>
        </item>
    </channel>
</rss>"#;

pub const FEED_WITH_IMAGES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Visual Blog</title>
        <description>Blog with images and media</description>
        <link>https://visual.example.com</link>
        <image>
            <url>https://visual.example.com/logo.png</url>
            <title>Visual Blog</title>
            <link>https://visual.example.com</link>
            <width>144</width>
            <height>144</height>
        </image>
        
        <item>
            <title>Photography Tips</title>
            <link>https://visual.example.com/photography-tips</link>
            <description><![CDATA[
                <p>Here are some great photography tips:</p>
                <img src="https://visual.example.com/tip1.jpg" alt="Camera settings"/>
                <p>Always check your camera settings before shooting.</p>
                <img src="https://visual.example.com/tip2.jpg" alt="Composition example"/>
                <p>Consider the rule of thirds for better composition.</p>
            ]]></description>
            <pubDate>Thu, 16 Mar 2024 16:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#;

pub const REDDIT_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>r/programming</title>
        <description>Programming discussions from Reddit</description>
        <link>https://www.reddit.com/r/programming</link>
        
        <item>
            <title>Show HN: I built a new RSS reader</title>
            <link>https://www.reddit.com/r/programming/comments/abc123</link>
            <description>I spent the last few months building a new RSS reader with modern features...</description>
            <author>u/developer123</author>
            <pubDate>Thu, 16 Mar 2024 20:00:00 GMT</pubDate>
            <comments>https://www.reddit.com/r/programming/comments/abc123</comments>
        </item>
        
        <item>
            <title>Ask HN: Best practices for REST API design?</title>
            <link>https://www.reddit.com/r/programming/comments/def456</link>
            <description>What are your go-to principles when designing REST APIs?</description>
            <author>u/apidesigner</author>
            <pubDate>Thu, 16 Mar 2024 19:00:00 GMT</pubDate>
            <comments>https://www.reddit.com/r/programming/comments/def456</comments>
        </item>
    </channel>
</rss>"#;

pub const GITHUB_RELEASES_ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
    <id>tag:github.com,2008:https://github.com/rust-lang/rust/releases</id>
    <link type="text/html" rel="alternate" href="https://github.com/rust-lang/rust/releases"/>
    <link type="application/atom+xml" rel="self" href="https://github.com/rust-lang/rust/releases.atom"/>
    <title>Release notes from rust</title>
    <updated>2024-03-16T10:00:00Z</updated>
    
    <entry>
        <id>tag:github.com,2008:Repository/724712/1.76.0</id>
        <updated>2024-03-16T10:00:00Z</updated>
        <link rel="alternate" type="text/html" href="https://github.com/rust-lang/rust/releases/tag/1.76.0"/>
        <title>Rust 1.76.0</title>
        <content type="html"><![CDATA[
            <h2>What's New in Rust 1.76.0</h2>
            <ul>
                <li>Improved error messages</li>
                <li>Performance optimizations</li>
                <li>New standard library features</li>
            </ul>
            <p>Download the latest version from the official website.</p>
        ]]></content>
        <author>
            <name>rust-lang</name>
        </author>
        <media:thumbnail height="30" width="30" url="https://avatars.githubusercontent.com/u/5430905?s=60&amp;v=4"/>
    </entry>
    
    <entry>
        <id>tag:github.com,2008:Repository/724712/1.75.0</id>
        <updated>2024-02-15T10:00:00Z</updated>
        <link rel="alternate" type="text/html" href="https://github.com/rust-lang/rust/releases/tag/1.75.0"/>
        <title>Rust 1.75.0</title>
        <content type="html"><![CDATA[
            <h2>What's New in Rust 1.75.0</h2>
            <ul>
                <li>New language features</li>
                <li>Stabilized APIs</li>
                <li>Compiler improvements</li>
            </ul>
        ]]></content>
        <author>
            <name>rust-lang</name>
        </author>
        <media:thumbnail height="30" width="30" url="https://avatars.githubusercontent.com/u/5430905?s=60&amp;v=4"/>
    </entry>
</feed>"#;