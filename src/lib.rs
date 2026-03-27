//! TTC Box Trading CLI
//!
//! Execute trading operations on TTC Box across 15+ exchanges.
//! Place orders, manage positions, set leverage, and control risk.

pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod crypto;
pub mod error;
pub mod models;
pub mod output;

pub use error::{Result, TtcError};
pub use config::AppConfig;
