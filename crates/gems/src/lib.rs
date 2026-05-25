/// Contract for all Viontin Gems.
///
/// Every Gem that can be registered via `boot().gem(...)` MUST implement
/// this trait. The `load()` method is the standard constructor — a Gem
/// is always instantiated through `SomeGem::load()`, optionally chaining
/// builder methods for configuration.
pub trait GemBuilder: Sized {
    fn load() -> Self;
}

// Re-export core types so gems can use them without deep deps
pub use viontin_core::{InternalResult, InternalError};

pub use viontin_framework::gem::*;
