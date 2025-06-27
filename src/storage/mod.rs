pub mod cache;
pub mod traits;
pub mod repository;
pub mod persistent_cache;

pub use cache::{
    ArticleCache, FeedCache, CacheManager, CacheConfig, CacheStats, CacheEntry
};
pub use persistent_cache::{PersistentCache, PersistentCacheConfig};
pub use traits::{
    Storage, Cache, FeedRepository, ArticleRepository,
    StorageStats, RepositoryStats, ArticleStats, ArticleQuery,
    StorageConfig, HealthStatus, CleanupStats, MemoryStorage
};
pub use repository::{Repository, RepositoryFactory};