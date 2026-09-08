//! Rust client for the [Ascii Box Public API v1](https://docs.ascii.dev/box/api/v1).
//!
//! Implements the documented supported subset of TypeScript `@asciidev/box-sdk`:
//! account, lifecycle, commands, prompts/events, files, desktop, snapshots,
//! environments, and bounded polling. See `docs/parity.md` for exact coverage.
//!
//! ```no_run
//! use box_client::{BoxApi, Configuration, CreateBoxRequest, wait_until_ready};
//!
//! # async fn demo() -> box_client::Result<()> {
//! let api = BoxApi::new(Configuration::from_env()?)?;
//! let created = api.create(CreateBoxRequest::ttl(1800)).await?;
//! wait_until_ready(&api, &created.box_.id).await?;
//! let out = api.exec(&created.box_.id, "uname -a").await?;
//! println!("{}", out.stdout);
//! api.stop(&created.box_.id, None).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod config;
mod error;
mod helpers;
mod types;
mod wait;

pub use client::BoxApi;
pub use config::Configuration;
pub use error::{Error, Result};
pub use helpers::{exec_command, read_text, stop_and_remove, stop_and_remove_with, write_text};
pub use types::*;
pub use wait::{
    wait_for_deletion, wait_for_desktop, wait_for_desktop_with, wait_for_prompt, wait_until_idle,
    wait_until_idle_with, wait_until_ready, wait_until_ready_with, DesktopWaitOptions, WaitOptions,
};

/// TypeScript `waitForPromptDone` alias.
pub use wait::wait_for_prompt as wait_for_prompt_done;
