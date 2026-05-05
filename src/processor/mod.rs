pub use engine::Processor;
pub(crate) use model::{CompactGroup, ItemSegment, NormalizedFile, PROMOTED_COMMENT_MARKER};

mod engine;
mod io;
mod model;
mod ordering;
mod render;
mod segment;
mod text;
#[cfg(test)]
mod tests;
