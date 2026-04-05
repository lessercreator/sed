pub mod schema;
pub mod types;
pub mod document;
pub mod query;
pub mod examples;
pub mod examples_office;
pub mod geometry;
pub mod undo;
pub mod validate;
pub mod diff;
pub mod import;
pub mod nlq;
pub mod clipboard;
pub mod autosize;
pub mod design_check;
mod tests;
mod bug_tests;

pub use document::SedDocument;
