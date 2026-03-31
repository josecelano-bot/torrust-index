//! V-tree violation tracking — violation sources configuration and the
//! violation-push queue used to collect violated nodes during mutations.

pub mod queue;
pub mod sources;

pub use queue::ViolationQueue;
