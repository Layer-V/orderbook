/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Journal trait and in-memory implementation for sequencer event storage.
//!
//! A journal is an append-only log of [`SequencerEvent`]s. It enables
//! deterministic replay, disaster recovery, and audit compliance by
//! preserving the full command history of an [`OrderBook`].
//!
//! [`OrderBook`]: crate::OrderBook

use super::event::SequencerEvent;
use super::replay::ReplayError;

/// Append-only event log for [`SequencerEvent`]s.
///
/// Implementations must preserve insertion order and provide efficient
/// iteration from an arbitrary sequence number. The journal is the
/// source of truth for [`ReplayEngine`] operations.
///
/// [`ReplayEngine`]: super::replay::ReplayEngine
pub trait Journal<T> {
    /// Appends a new event to the journal.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError`] if the event cannot be stored (e.g. sequence
    /// number out of order or storage failure).
    fn append(&mut self, event: SequencerEvent<T>) -> Result<(), ReplayError>;

    /// Returns an iterator over all events with `sequence_num >= from_sequence`.
    ///
    /// Events are yielded in ascending sequence order.
    fn read_from(&self, from_sequence: u64) -> impl Iterator<Item = &SequencerEvent<T>> + '_
    where
        T: 'static;

    /// Returns an iterator over events with `from_sequence <= sequence_num <= to_sequence`.
    ///
    /// Events are yielded in ascending sequence order.
    fn read_range(
        &self,
        from_sequence: u64,
        to_sequence: u64,
    ) -> impl Iterator<Item = &SequencerEvent<T>> + '_
    where
        T: 'static;

    /// Returns the total number of events stored.
    #[must_use]
    fn len(&self) -> usize;

    /// Returns `true` if no events have been appended.
    #[must_use]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the sequence number of the last event, or `None` if empty.
    #[must_use]
    fn last_sequence(&self) -> Option<u64>;
}

/// In-memory implementation of [`Journal`].
///
/// Stores all events in a `Vec` in insertion order. Suitable for testing,
/// benchmarking, and short-lived workloads where persistence is not required.
///
/// # Examples
///
/// ```
/// use orderbook_rs::sequencer::journal::{Journal, InMemoryJournal};
/// use orderbook_rs::sequencer::{SequencerCommand, SequencerEvent, SequencerResult};
/// use pricelevel::OrderId;
///
/// let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
/// assert!(journal.is_empty());
///
/// let event = SequencerEvent::new(
///     1,
///     0,
///     SequencerCommand::CancelOrder(OrderId::new()),
///     SequencerResult::OrderCancelled { order_id: OrderId::new() },
/// );
/// journal.append(event).ok();
/// assert_eq!(journal.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct InMemoryJournal<T> {
    events: Vec<SequencerEvent<T>>,
}

impl<T> InMemoryJournal<T> {
    /// Creates a new empty in-memory journal.
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Creates a new in-memory journal with pre-allocated capacity.
    ///
    /// Use this when the approximate number of events is known in advance
    /// to avoid repeated reallocations.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
        }
    }

    /// Returns a slice of all stored events.
    #[must_use]
    pub fn events(&self) -> &[SequencerEvent<T>] {
        &self.events
    }
}

impl<T: Clone + Send + Sync + Default + 'static> Journal<T> for InMemoryJournal<T> {
    fn append(&mut self, event: SequencerEvent<T>) -> Result<(), ReplayError> {
        self.events.push(event);
        Ok(())
    }

    fn read_from(&self, from_sequence: u64) -> impl Iterator<Item = &SequencerEvent<T>> + '_ {
        self.events
            .iter()
            .filter(move |e| e.sequence_num >= from_sequence)
    }

    fn read_range(
        &self,
        from_sequence: u64,
        to_sequence: u64,
    ) -> impl Iterator<Item = &SequencerEvent<T>> + '_ {
        self.events
            .iter()
            .filter(move |e| e.sequence_num >= from_sequence && e.sequence_num <= to_sequence)
    }

    #[inline]
    fn len(&self) -> usize {
        self.events.len()
    }

    #[inline]
    fn last_sequence(&self) -> Option<u64> {
        self.events.last().map(|e| e.sequence_num)
    }
}
