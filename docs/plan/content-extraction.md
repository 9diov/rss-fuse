# Content Extraction Algorithms

## Current Implementation Status

- ✅ **Content Module** (`src/content/mod.rs`): Content extraction interface
  - ✅ CSS selector patterns for article extraction
  - ✅ Content cleaning selectors
  - ✅ Public API exports
- ✅ **Extractor Module** (`src/content/extractor.rs`): Complete HTML to Markdown conversion
  - ✅ HTML to Markdown conversion using `html2md`
  - ✅ YAML frontmatter generation with structured metadata
  - ✅ Content cleaning and post-processing
  - ✅ Category extraction from content
  - ✅ Unit tests passing (6/6 tests)

## Development Plan

### Phase 1: Basic Content Processing ✅ COMPLETED
- ✅ **HTML to Text**: Convert HTML articles to readable plain text
- ✅ **Content Cleaning**: Remove ads, navigation, and boilerplate content
- ✅ **Markdown Formatting**: Convert articles to proper Markdown format with:
  - ✅ **YAML Frontmatter**: Include metadata (title, author, date, url, tags, categories)
  - ✅ **Heading Structure**: Preserve article headings as Markdown headers
  - ✅ **List Formatting**: Convert HTML lists to Markdown lists
  - ✅ **Link Formatting**: Convert links to Markdown syntax `[text](url)`
  - ✅ **Image References**: Convert images to Markdown syntax `![alt](url)`
  - ✅ **Code Blocks**: Detect and format code snippets with proper fencing
  - ✅ **Blockquotes**: Preserve quotes with Markdown blockquote syntax
- ✅ **Link Extraction**: Extract and preserve important links within content
- ✅ **Image Handling**: Reference images with URLs or download locally

### Phase 2: Advanced Extraction
- [ ] **Readability Algorithm**: Implement content extraction similar to Firefox Reader
- [ ] **Language Detection**: Detect article language for better processing
- [ ] **Encoding Handling**: Properly handle various character encodings
- [ ] **Media Extraction**: Extract video and audio embed codes
- [ ] **Table Processing**: Preserve table structure in text format

### Phase 3: Content Enhancement
- [ ] **Summary Generation**: Auto-generate article summaries
- [ ] **Keyword Extraction**: Extract relevant keywords and tags
- [ ] **Content Classification**: Categorize articles by topic
- [ ] **Duplicate Content**: Detect and merge duplicate articles across feeds
- [ ] **Content Archival**: Long-term storage of article snapshots

## Article Output Format

Articles are now saved as Markdown files (`.md`) with YAML frontmatter:

```markdown
---
title: "Article Title Here"
author: "Author Name"
date: 2024-01-15T10:30:00Z
url: "https://original-article-url.com"
feed: "hacker-news"
tags: ["rust", "programming", "tutorial"]
categories: ["technology", "development"]
description: "Brief article summary or excerpt"
guid: "article-unique-identifier"
---

# Article Title Here

Article content in proper Markdown format with:

## Preserved Section Headers

- **Bold text** and *italic text*
- Properly formatted lists
- Nested lists with proper indentation

1. Numbered lists
2. Also preserved

> Blockquotes for quotations and excerpts

`inline code` and code blocks:

```rust
// Code blocks with language hints for syntax highlighting
fn main() {
    println!("Hello, world!");
}
```

Links formatted as [descriptive text](https://example.com) and images:

![Alt text](https://example.com/image.png)
```

## Benefits of Markdown Format

- **Better Readability**: Markdown is more readable than plain text
- **Metadata Access**: YAML frontmatter provides structured access to article data
- **Tool Compatibility**: Markdown files work with static site generators, note-taking apps
- **Syntax Highlighting**: Code blocks can be highlighted by viewers
- **Future Extensibility**: Easy to add new metadata fields