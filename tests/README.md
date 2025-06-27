# RSS-FUSE Test Suite

This directory contains comprehensive tests for the RSS feed parsing and fetching functionality.

## Test Structure

### Unit Tests

#### `src/feed/parser.rs`
- **RSS 2.0 parsing**: Valid RSS feeds with various elements
- **Atom 1.0 parsing**: Complete Atom feed processing
- **Malformed XML handling**: Error recovery for broken feeds
- **Empty feeds**: Graceful handling of feeds with no content
- **HTML entities**: Proper decoding of XML entities
- **CDATA sections**: Support for CDATA content blocks
- **URL validation**: Comprehensive URL scheme and format validation

#### `src/feed/fetcher.rs`
- **HTTP client functionality**: GET requests with proper headers
- **Error handling**: 404s, timeouts, server errors
- **Content negotiation**: Accept headers for RSS/Atom formats
- **Redirects**: Following HTTP redirects properly
- **Compression**: Gzip/deflate response handling
- **Concurrent fetching**: Multiple feeds in parallel
- **Large feeds**: Memory-efficient processing of large feeds
- **Rate limiting**: Respectful request patterns

### Integration Tests

#### `tests/feed_integration_tests.rs`
- **End-to-end RSS processing**: Complete workflow from fetch to article creation
- **End-to-end Atom processing**: Full Atom feed pipeline
- **Concurrent feed fetching**: Real-world concurrent scenarios
- **Error handling and recovery**: Comprehensive error scenario testing
- **Special characters and encoding**: Unicode and international content
- **Article ID generation**: Consistent ID creation and deduplication
- **Feed caching headers**: ETag and Last-Modified support
- **Content negotiation**: Accept header verification
- **Large feed memory efficiency**: Memory usage optimization
- **Relative URLs**: URL resolution handling
- **Custom namespaces**: Extended RSS/Atom namespace support

### Property-Based Tests

#### `tests/property_tests.rs`
- **Filename generation safety**: Ensures safe filesystem names
- **URL validation robustness**: Tests with random URL patterns
- **Article ID consistency**: Deterministic ID generation
- **Article text generation safety**: Safe text output
- **RSS parsing with random valid structure**: Fuzz testing valid RSS
- **Memory safety with large content**: Stress testing with large inputs
- **HTML entity handling**: Random entity combinations
- **Article deduplication consistency**: Consistent duplicate detection

### Performance Tests

#### `benches/feed_benchmarks.rs`
- **Feed parsing performance**: Benchmarks for different feed types
- **Large feed parsing**: Performance with increasing article counts
- **Article creation**: Benchmark article object creation
- **Concurrent parsing**: Parallel vs sequential processing
- **URL validation**: URL checking performance
- **Memory usage**: Memory efficiency with large content

## Test Data

### `tests/test_data.rs` and `benches/test_data.rs`
Contains sample RSS and Atom feeds for testing:

- **Tech News RSS**: Multi-article RSS with categories and metadata
- **Science Blog Atom**: Atom feed with HTML content
- **Unicode RSS**: International characters and emojis
- **Namespaced RSS**: Custom XML namespaces
- **Podcast RSS**: iTunes podcast-specific elements
- **GitHub Releases Atom**: Real-world Atom feed structure
- **Reddit RSS**: Social media feed format
- **Malformed XML**: Various broken XML scenarios

## Running Tests

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test feed_integration_tests
```

### Property-Based Tests
```bash
cargo test --test property_tests
```

### Benchmarks
```bash
cargo bench
```

### All Tests with Coverage
```bash
cargo test --all-features
```

### Specific Test Categories
```bash
# Parser tests only
cargo test parser

# Fetcher tests only  
cargo test fetcher

# Error handling tests
cargo test error

# Unicode/internationalization tests
cargo test unicode

# Performance critical tests
cargo test concurrent
```

## Test Configuration

### Environment Variables
- `RUST_LOG=debug`: Enable debug logging during tests
- `TEST_TIMEOUT=30`: Set custom timeout for network tests (seconds)
- `SKIP_NETWORK_TESTS=1`: Skip tests requiring network access

### Features
- `--features metrics`: Include metrics collection tests
- `--features vendored-sqlite`: Use bundled SQLite for tests

## Writing New Tests

### Guidelines
1. **Test naming**: Use descriptive names starting with `test_`
2. **Error testing**: Always test both success and failure cases
3. **Edge cases**: Include empty, null, and boundary value tests
4. **Performance**: Add benchmarks for performance-critical code
5. **Documentation**: Document complex test scenarios

### Mock Server Usage
Tests use `wiremock` for HTTP mocking:

```rust
let mock_server = MockServer::start().await;
Mock::given(method("GET"))
    .and(path("/feed.xml"))
    .respond_with(ResponseTemplate::new(200).set_body_string(RSS_CONTENT))
    .mount(&mock_server)
    .await;
```

### Property Test Strategies
Use `proptest` for generating test inputs:

```rust
proptest! {
    #[test]
    fn test_property(input in strategy()) {
        // Test logic here
        prop_assert!(condition);
    }
}
```

## Common Test Patterns

### Testing Error Conditions
```rust
let result = parser.parse_feed(malformed_input);
assert!(result.is_err());
match result {
    Err(Error::FeedParse(msg)) => assert!(msg.contains("expected error")),
    _ => panic!("Wrong error type"),
}
```

### Testing Async Functions
```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Testing Large Data
```rust
#[test]
fn test_large_feed() {
    let large_feed = create_feed_with_articles(10000);
    let result = parser.parse_feed(large_feed);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().articles.len(), 10000);
}
```

## Debugging Tests

### Failed Tests
```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_specific_function -- --exact

# Show ignored tests
cargo test -- --ignored
```

### Benchmark Analysis
```bash
# Generate HTML report
cargo bench
open target/criterion/report/index.html
```

### Memory Profiling
```bash
# With valgrind (Linux)
cargo test --target x86_64-unknown-linux-gnu
valgrind --tool=massif target/debug/deps/test_binary

# With instruments (macOS)
cargo test --release
instruments -t Allocations target/release/deps/test_binary
```

## Continuous Integration

Tests are designed to run in CI environments:
- **Fast execution**: Most tests complete under 30 seconds
- **Deterministic**: No flaky tests or race conditions
- **Isolated**: Each test is independent
- **Resource efficient**: Reasonable memory and CPU usage

### CI Configuration
```yaml
- name: Run tests
  run: |
    cargo test --all-features
    cargo test --release
    cargo bench --no-run
```