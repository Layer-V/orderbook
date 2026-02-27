/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Sequencer receipt types.
//!
//! This module defines the receipt returned to callers after submitting
//! a command to the Sequencer.

use super::result::SequencerResult;

/// Receipt returned after submitting a command to the Sequencer.
///
/// Contains the assigned sequence number and the result of executing
/// the command.
///
/// # Examples
///
/// ```
/// use orderbook_rs::sequencer::SequencerReceipt;
/// # use orderbook_rs::sequencer::SequencerResult;
/// # use pricelevel::OrderId;
///
/// # let receipt = SequencerReceipt {
/// #     sequence_num: 42,
/// #     result: SequencerResult::OrderAdded { order_id: OrderId::new() },
/// # };
/// assert_eq!(receipt.sequence_num, 42);
/// assert!(receipt.result.is_success());
/// ```
#[derive(Debug)]
pub struct SequencerReceipt {
    /// The monotonically increasing sequence number assigned to this command.
    pub sequence_num: u64,

    /// The result of executing the command.
    pub result: SequencerResult,
}

impl SequencerReceipt {
    /// Creates a new receipt.
    #[must_use]
    pub fn new(sequence_num: u64, result: SequencerResult) -> Self {
        Self {
            sequence_num,
            result,
        }
    }

    /// Returns `true` if the command was successful.
    #[inline]
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.result.is_success()
    }
}
