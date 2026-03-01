/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Deterministic replay engine for event journals.
//!
//! [`ReplayEngine`] reads a sequence of [`SequencerEvent`]s from a [`Journal`]
//! and re-applies each command to a fresh [`OrderBook`], producing an
//! identical final state. This enables disaster recovery, audit compliance,
//! and state verification.
//!
//! # Examples
//!
//! ```no_run
//! use orderbook_rs::sequencer::journal::{Journal, InMemoryJournal};
//! use orderbook_rs::sequencer::replay::ReplayEngine;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let journal: InMemoryJournal<()> = InMemoryJournal::new();
//! let (book, last_seq) = ReplayEngine::replay_from(&journal, 0, "BTC/USD")?;
//! println!("Replayed up to sequence {last_seq}");
//! # Ok(())
//! # }
//! ```

use super::command::SequencerCommand;
use super::event::SequencerEvent;
use super::journal::Journal;
use crate::orderbook::{OrderBook, OrderBookError, OrderBookSnapshot};
use std::marker::PhantomData;
use thiserror::Error;

/// Errors that can occur during journal replay.
#[derive(Debug, Error)]
pub enum ReplayError {
    /// The journal contains no events to replay.
    #[error("journal is empty — nothing to replay")]
    EmptyJournal,

    /// The requested starting sequence number exceeds the journal's last entry.
    #[error("invalid from_sequence {from_sequence}: journal last sequence is {last_sequence}")]
    InvalidSequence {
        /// The sequence number requested.
        from_sequence: u64,
        /// The last sequence number in the journal.
        last_sequence: u64,
    },

    /// A gap was detected between expected and found sequence numbers.
    #[error("sequence gap detected: expected {expected}, found {found}")]
    SequenceGap {
        /// The expected next sequence number.
        expected: u64,
        /// The actual sequence number found.
        found: u64,
    },

    /// An OrderBook operation failed during replay.
    #[error("order book error during replay at sequence {sequence_num}: {source}")]
    OrderBookError {
        /// The sequence number of the event that caused the error.
        sequence_num: u64,
        /// The underlying error.
        #[source]
        source: OrderBookError,
    },

    /// The replayed state does not match the expected snapshot.
    #[error("snapshot mismatch: replayed state diverges from expected snapshot")]
    SnapshotMismatch,
}

/// Stateless replay engine that reconstructs [`OrderBook`] state from a [`Journal`].
///
/// All methods are associated functions (no `&self` receiver) — `ReplayEngine`
/// holds no state itself. Use it as a namespace for replay operations.
///
/// # Examples
///
/// ```no_run
/// use orderbook_rs::sequencer::journal::{Journal, InMemoryJournal};
/// use orderbook_rs::sequencer::replay::ReplayEngine;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let journal: InMemoryJournal<()> = InMemoryJournal::new();
/// let (book, last_seq) = ReplayEngine::replay_from(&journal, 0, "BTC/USD")?;
/// # Ok(())
/// # }
/// ```
pub struct ReplayEngine<T> {
    _phantom: PhantomData<T>,
}

impl<T: Clone + Send + Sync + Default + 'static> ReplayEngine<T> {
    /// Replays all events from `from_sequence` onwards onto a fresh [`OrderBook`].
    ///
    /// Returns the reconstructed book and the sequence number of the last
    /// event applied. Only successful commands (non-`Rejected` results) are
    /// replayed — rejected events are skipped without error.
    ///
    /// # Arguments
    ///
    /// * `journal` — the event source
    /// * `from_sequence` — first sequence number to include (inclusive); pass `0` for full replay
    /// * `symbol` — symbol used to create the fresh OrderBook
    ///
    /// # Errors
    ///
    /// - [`ReplayError::EmptyJournal`] if the journal has no events
    /// - [`ReplayError::InvalidSequence`] if `from_sequence` > last journal sequence
    /// - [`ReplayError::OrderBookError`] if a command fails unexpectedly during replay
    pub fn replay_from(
        journal: &impl Journal<T>,
        from_sequence: u64,
        symbol: &str,
    ) -> Result<(OrderBook<T>, u64), ReplayError> {
        Self::replay_from_with_progress(journal, from_sequence, symbol, |_, _| {})
    }

    /// Replays events with a progress callback invoked after each applied event.
    ///
    /// The callback receives `(events_applied: u64, current_sequence: u64)`.
    /// Useful for long replays where progress reporting is needed.
    ///
    /// # Arguments
    ///
    /// * `journal` — the event source
    /// * `from_sequence` — first sequence number to include; pass `0` for full replay
    /// * `symbol` — symbol for the fresh OrderBook
    /// * `progress` — callback invoked after each event: `(events_applied, sequence_num)`
    ///
    /// # Errors
    ///
    /// Same as [`replay_from`](Self::replay_from).
    pub fn replay_from_with_progress(
        journal: &impl Journal<T>,
        from_sequence: u64,
        symbol: &str,
        progress: impl Fn(u64, u64),
    ) -> Result<(OrderBook<T>, u64), ReplayError> {
        if journal.is_empty() {
            return Err(ReplayError::EmptyJournal);
        }

        if journal
            .last_sequence()
            .is_some_and(|last| from_sequence > last)
        {
            return Err(ReplayError::InvalidSequence {
                from_sequence,
                last_sequence: journal.last_sequence().unwrap_or(0),
            });
        }

        let book = OrderBook::new(symbol);
        let mut last_seq = 0u64;
        let mut count = 0u64;

        for event in journal.read_from(from_sequence) {
            Self::apply_event(&book, event)?;
            last_seq = event.sequence_num;
            count = count.saturating_add(1);
            progress(count, last_seq);
        }

        Ok((book, last_seq))
    }

    /// Returns the events with `from_sequence <= sequence_num <= to_sequence`.
    ///
    /// No OrderBook is constructed — this is a pure slice of the journal.
    /// Useful for auditing, debugging, or feeding events to external consumers.
    ///
    /// # Errors
    ///
    /// - [`ReplayError::EmptyJournal`] if the journal has no events
    /// - [`ReplayError::InvalidSequence`] if `from_sequence` > last journal sequence
    #[must_use = "returns the event slice — use it or it is wasted work"]
    pub fn replay_range(
        journal: &impl Journal<T>,
        from_sequence: u64,
        to_sequence: u64,
    ) -> Result<Vec<&SequencerEvent<T>>, ReplayError> {
        if journal.is_empty() {
            return Err(ReplayError::EmptyJournal);
        }

        if journal
            .last_sequence()
            .is_some_and(|last| from_sequence > last)
        {
            return Err(ReplayError::InvalidSequence {
                from_sequence,
                last_sequence: journal.last_sequence().unwrap_or(0),
            });
        }

        Ok(journal.read_range(from_sequence, to_sequence).collect())
    }

    /// Replays the full journal and compares the result to an expected snapshot.
    ///
    /// Returns `Ok(true)` if the replayed state matches, `Ok(false)` if it
    /// diverges. The comparison uses [`snapshots_match`] which checks symbol,
    /// bid price levels, and ask price levels.
    ///
    /// # Errors
    ///
    /// - [`ReplayError::EmptyJournal`] if the journal has no events
    /// - [`ReplayError::OrderBookError`] if replay fails
    pub fn verify(
        journal: &impl Journal<T>,
        expected_snapshot: &OrderBookSnapshot,
    ) -> Result<bool, ReplayError> {
        let (book, _) = Self::replay_from(journal, 0, &expected_snapshot.symbol)?;
        let actual = book.create_snapshot(usize::MAX);
        Ok(snapshots_match(&actual, expected_snapshot))
    }

    /// Applies a single sequencer event to the given book.
    ///
    /// Events with `Rejected` results are skipped — they represent commands
    /// that failed at write time and must not be re-applied during replay.
    fn apply_event(book: &OrderBook<T>, event: &SequencerEvent<T>) -> Result<(), ReplayError> {
        // Skip events whose original execution was rejected.
        if event.result.is_rejected() {
            return Ok(());
        }

        match &event.command {
            SequencerCommand::AddOrder(order) => {
                book.add_order(order.clone())
                    .map_err(|e| ReplayError::OrderBookError {
                        sequence_num: event.sequence_num,
                        source: e,
                    })?;
            }
            SequencerCommand::CancelOrder(id) => {
                // cancel_order returns Ok(None) when the order was already
                // removed by a prior cancel. This is not an error during
                // replay — we tolerate it silently.
                book.cancel_order(*id)
                    .map_err(|e| ReplayError::OrderBookError {
                        sequence_num: event.sequence_num,
                        source: e,
                    })?;
            }
        }

        Ok(())
    }
}

/// Compares two [`OrderBookSnapshot`]s for structural equality.
///
/// Two snapshots are considered equal when:
/// - `symbol` is identical
/// - The sorted bid price levels match (by price, then visible quantity)
/// - The sorted ask price levels match (by price, then visible quantity)
///
/// Timestamps are intentionally excluded from comparison because replayed
/// books may be created at a different wall-clock time than the original.
///
/// # Examples
///
/// ```
/// use orderbook_rs::sequencer::replay::snapshots_match;
/// use orderbook_rs::OrderBookSnapshot;
/// use pricelevel::PriceLevelSnapshot;
///
/// let a = OrderBookSnapshot {
///     symbol: "BTC/USD".to_string(),
///     timestamp: 0,
///     bids: vec![],
///     asks: vec![],
/// };
/// let b = OrderBookSnapshot {
///     symbol: "BTC/USD".to_string(),
///     timestamp: 999,
///     bids: vec![],
///     asks: vec![],
/// };
/// assert!(snapshots_match(&a, &b));
/// ```
#[must_use]
pub fn snapshots_match(actual: &OrderBookSnapshot, expected: &OrderBookSnapshot) -> bool {
    if actual.symbol != expected.symbol {
        return false;
    }

    // Compare bids sorted by price descending (highest bid first)
    let mut actual_bids: Vec<_> = actual.bids.iter().collect();
    let mut expected_bids: Vec<_> = expected.bids.iter().collect();
    actual_bids.sort_by(|a, b| b.price.cmp(&a.price));
    expected_bids.sort_by(|a, b| b.price.cmp(&a.price));

    if actual_bids.len() != expected_bids.len() {
        return false;
    }
    for (a, b) in actual_bids.iter().zip(expected_bids.iter()) {
        if a.price != b.price || a.visible_quantity != b.visible_quantity {
            return false;
        }
    }

    // Compare asks sorted by price ascending (lowest ask first)
    let mut actual_asks: Vec<_> = actual.asks.iter().collect();
    let mut expected_asks: Vec<_> = expected.asks.iter().collect();
    actual_asks.sort_by_key(|l| l.price);
    expected_asks.sort_by_key(|l| l.price);

    if actual_asks.len() != expected_asks.len() {
        return false;
    }
    for (a, b) in actual_asks.iter().zip(expected_asks.iter()) {
        if a.price != b.price || a.visible_quantity != b.visible_quantity {
            return false;
        }
    }

    true
}
