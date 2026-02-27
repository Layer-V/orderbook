/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Sequencer event types.
//!
//! This module defines the events emitted by the Sequencer after executing
//! each command.

use super::command::SequencerCommand;
use super::result::SequencerResult;

/// Event emitted after executing a sequencer command.
///
/// Contains the sequence number, timestamp, original command, and result.
/// Events are emitted in sequence order and can be used for replay,
/// auditing, or real-time monitoring.
///
/// # Examples
///
/// ```
/// use orderbook_rs::sequencer::SequencerEvent;
/// # use orderbook_rs::sequencer::{SequencerCommand, SequencerResult};
/// # use pricelevel::OrderId;
///
/// # let event: SequencerEvent<()> = SequencerEvent {
/// #     sequence_num: 1,
/// #     timestamp_ns: 1234567890,
/// #     command: SequencerCommand::CancelOrder(OrderId::new()),
/// #     result: SequencerResult::OrderCancelled { order_id: OrderId::new() },
/// # };
/// assert_eq!(event.sequence_num, 1);
/// ```
#[derive(Debug)]
pub struct SequencerEvent<T> {
    /// Monotonically increasing sequence number.
    pub sequence_num: u64,

    /// Nanosecond timestamp when the command was executed.
    pub timestamp_ns: u64,

    /// The command that was executed.
    pub command: SequencerCommand<T>,

    /// The result of executing the command.
    pub result: SequencerResult,
}

impl<T> SequencerEvent<T> {
    /// Creates a new sequencer event.
    #[must_use]
    pub fn new(
        sequence_num: u64,
        timestamp_ns: u64,
        command: SequencerCommand<T>,
        result: SequencerResult,
    ) -> Self {
        Self {
            sequence_num,
            timestamp_ns,
            command,
            result,
        }
    }
}
