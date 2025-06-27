# Feed Parsing and Fetching Logic

## Current Implementation Status

- ✅ **Parser Module** (`src/feed/parser.rs`): RSS 2.0 and Atom 1.0 support using `feed-rs` crate
  - ✅ Unit tests passing (9/9 tests)
  - ✅ URL validation implemented
  - ✅ Malformed XML handling
  - ✅ HTML entity decoding
  - ✅ CDATA section support
- ✅ **Fetcher Module** (`src/feed/fetcher.rs`): HTTP client with reqwest for concurrent feed downloads
  - ✅ HTTP client with timeouts and redirects
  - ✅ User-agent configuration
  - ✅ Error handling and retry logic
  - ✅ Multiple feed fetching support
- ✅ **Feed Model** (`src/feed/mod.rs`): Core data structures and article processing
  - ✅ Article ID generation with blake3 hashing
  - ✅ Text formatting for file output
  - ✅ Filename sanitization
- ❌ **Manager Module** (`src/feed/manager.rs`): Not yet implemented
- ❌ **Cache Module** (`src/feed/cache.rs`): Not yet implemented

## Development Plan

### Phase 1: Core Feed Operations ✅ COMPLETED
- ✅ **Feed URL Validation**: Implemented robust URL sanitization and validation
- ✅ **HTTP Client Configuration**: Setup with proper timeouts, retries, and user-agent
- ✅ **RSS/Atom Parser**: Handle malformed XML gracefully with error recovery
- ✅ **Content Extraction**: Basic content extraction from feed descriptions
- ✅ **Duplicate Detection**: Implemented article deduplication based on GUID/URL with blake3 hashing

### Phase 2: Advanced Features  
- [ ] **Feed Discovery**: Auto-detect RSS/Atom feeds from website URLs
- [ ] **Format Support**: Add JSON Feed and Reddit RSS support
- [ ] **Content Filtering**: Allow regex-based content filtering per feed
- [ ] **Rate Limiting**: Implement per-domain request throttling
- [ ] **Authentication**: Support for password-protected feeds

### Phase 3: Performance Optimization
- [ ] **Connection Pooling**: Reuse HTTP connections across requests
- [ ] **Conditional Requests**: Use ETags and Last-Modified headers
- [ ] **Compression**: Support gzip/deflate response encoding
- [ ] **Streaming Parser**: Handle large feeds without loading everything into memory
- [ ] **Batch Processing**: Group feed updates efficiently