/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Sequencer module for total ordering of order operations.
//!
//! This module provides a single-threaded Sequencer that wraps an OrderBook
//! and ensures all operations are executed in a deterministic, totally-ordered
//! sequence with monotonic sequence numbers. This follows the LMAX Disruptor
//! pattern for high-throughput, low-latency event processing.
//!
//! # Architecture
//!
//! - Commands are submitted via an async channel
//! - A single-threaded event loop processes commands in order
//! - Each command receives a monotonic sequence number and nanosecond timestamp
//! - Results are returned via oneshot channels
//! - Events are emitted to registered listeners in sequence order
//!
//! # Examples
//!
//! ```no_run
//! use orderbook_rs::sequencer::{Sequencer, SequencerCommand};
//! use orderbook_rs::DefaultOrderBook;
//! use pricelevel::OrderId;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
//!
//! // Register an event listener
//! sequencer.add_listener(|event| {
//!     println!("Event {}: {:?}", event.sequence_num, event.result);
//! });
//!
//! // Spawn the sequencer
//! let handle = sequencer.spawn();
//!
//! // Submit commands (from another task/thread)
//! // let receipt = sequencer.submit(command).await?;
//!
//! // Wait for shutdown
//! handle.wait().await?;
//! # Ok(())
//! # }
//! ```

pub mod command;
pub mod core;
pub mod event;
pub mod receipt;
pub mod result;

#[cfg(test)]
mod tests;

// Re-export main types
pub use command::SequencerCommand;
pub use core::{Sequencer, SequencerError, SequencerHandle};
pub use event::SequencerEvent;
pub use receipt::SequencerReceipt;
pub use result::SequencerResult;
