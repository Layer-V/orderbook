/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Sequencer result types.
//!
//! This module defines the result types returned after executing commands
//! on the Sequencer.

use crate::TradeResult;
use crate::orderbook::OrderBookError;
use pricelevel::OrderId;

/// Result of executing a sequencer command.
///
/// Indicates whether the command succeeded and what the outcome was.
#[derive(Debug)]
pub enum SequencerResult {
    /// Order was successfully added to the book.
    OrderAdded {
        /// ID of the added order.
        order_id: OrderId,
    },

    /// Order was successfully cancelled.
    OrderCancelled {
        /// ID of the cancelled order.
        order_id: OrderId,
    },

    /// Trade was executed (market order or matched limit order).
    TradeExecuted {
        /// Details of the trade execution.
        trade_result: TradeResult,
    },

    /// Command was rejected due to an error.
    Rejected {
        /// The error that caused rejection.
        error: OrderBookError,
    },
}

impl SequencerResult {
    /// Returns `true` if the command was successful.
    #[inline]
    #[must_use]
    pub fn is_success(&self) -> bool {
        !matches!(self, Self::Rejected { .. })
    }

    /// Returns `true` if the command was rejected.
    #[inline]
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}
