#![recursion_limit = "128"]

#[macro_use]
mod macros;

pub mod app;
pub mod errors;
pub mod path;
pub mod service;
pub mod terminal;

mod command;
mod config;
mod fs;
mod judging;
mod template;
mod testsuite;
mod time;
mod util;
mod yaml;

pub use crate::errors::{Error, ErrorKind, Result};