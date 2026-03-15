#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Append-only JSONL ledger storage with SHA-256 hash chaining.
mod hash;
mod model;
mod reader;
mod storage;
mod verify;
mod writer;

pub use model::{LedgerCursor, LedgerEntryDraft};
pub use reader::LedgerReader;
pub use verify::verify_chain;
pub use writer::LedgerWriter;
