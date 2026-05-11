//! Tetrac Trading CLI
//!
//! Execute trading operations on Tetrac across 15+ exchanges.
//! Place orders, manage positions, set leverage, and control risk.

pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod crypto;
pub mod error;
pub mod models;
pub mod output;

pub use config::AppConfig;
pub use error::{Result, TtcError};
