/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Sequencer command types.
//!
//! This module defines the commands that can be submitted to the Sequencer
//! for ordered execution on the OrderBook.

use pricelevel::{OrderId, OrderType};

/// Commands that can be submitted to the Sequencer.
///
/// Each command represents an operation to be executed on the OrderBook
/// in a deterministic, totally-ordered sequence.
///
/// # Examples
///
/// ```
/// use orderbook_rs::sequencer::SequencerCommand;
/// use pricelevel::OrderId;
///
/// let command: SequencerCommand<()> = SequencerCommand::CancelOrder(OrderId::new());
/// ```
#[derive(Debug, Clone)]
pub enum SequencerCommand<T> {
    /// Add a new order to the book.
    AddOrder(OrderType<T>),

    /// Cancel an existing order.
    CancelOrder(OrderId),
}
