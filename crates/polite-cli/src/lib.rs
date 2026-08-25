//! The PoliteLang toolchain, as a library.
//!
//! The `polite` command is a thin shell around this. Keeping it here means the test suites drive
//! exactly the same pipeline a person does.

#![forbid(unsafe_code)]

pub mod bench;
pub mod grammar;
pub mod pipeline;
pub mod words;
