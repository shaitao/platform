//!
//! Tendermint-based abci implementation
//!
//! 

#![deny(warnings)]
#![deny(missing_docs)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::field_reassign_with_default)]

#[macro_use]
extern crate lazy_static;

pub mod abci;
pub mod api;
