/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Core Sequencer implementation.
//!
//! This module provides the main Sequencer struct that wraps an OrderBook
//! and ensures all operations are executed in a deterministic, totally-ordered
//! sequence with monotonic sequence numbers.

use super::command::SequencerCommand;
use super::event::SequencerEvent;
use super::receipt::SequencerReceipt;
use super::result::SequencerResult;
use crate::orderbook::OrderBook;
use pricelevel::{OrderId, OrderType};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, oneshot};

/// Type alias for event listener functions.
type EventListener<T> = Arc<dyn Fn(&SequencerEvent<T>) + Send + Sync>;

/// A single-threaded sequencer that provides total ordering of order operations.
///
/// The Sequencer wraps an `OrderBook` and ensures all operations are executed
/// in a deterministic order with monotonically increasing sequence numbers.
/// This follows the LMAX Disruptor pattern: a single writer thread processes
/// all events in order, eliminating the need for locks.
///
/// # Examples
///
/// ```no_run
/// use orderbook_rs::sequencer::Sequencer;
/// use orderbook_rs::DefaultOrderBook;
///
/// # async fn example() {
/// let sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
/// let handle = sequencer.spawn();
/// // Submit commands...
/// # }
/// ```
pub struct Sequencer<T: Clone + Send + Sync + Default + 'static> {
    /// The underlying order book.
    book: OrderBook<T>,

    /// Monotonic sequence counter.
    sequence: Arc<AtomicU64>,

    /// Channel for submitting commands.
    command_tx: mpsc::Sender<(SequencerCommand<T>, oneshot::Sender<SequencerReceipt>)>,

    /// Channel for receiving commands (used by event loop).
    command_rx: Option<mpsc::Receiver<(SequencerCommand<T>, oneshot::Sender<SequencerReceipt>)>>,

    /// Event listeners called synchronously for each event.
    event_listeners: Vec<EventListener<T>>,
}

impl<T: Clone + Send + Sync + Default + 'static> Sequencer<T> {
    /// Creates a new Sequencer wrapping the given OrderBook.
    ///
    /// # Arguments
    ///
    /// * `book` - The OrderBook to wrap
    ///
    /// # Examples
    ///
    /// ```
    /// use orderbook_rs::sequencer::Sequencer;
    /// use orderbook_rs::DefaultOrderBook;
    ///
    /// let sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
    /// ```
    #[must_use]
    pub fn new(book: OrderBook<T>) -> Self {
        Self::with_capacity(book, 65536)
    }

    /// Creates a new Sequencer with a specific channel capacity.
    ///
    /// # Arguments
    ///
    /// * `book` - The OrderBook to wrap
    /// * `capacity` - Channel buffer size (backpressure when full)
    #[must_use]
    pub fn with_capacity(book: OrderBook<T>, capacity: usize) -> Self {
        let (command_tx, command_rx) = mpsc::channel(capacity);

        Self {
            book,
            sequence: Arc::new(AtomicU64::new(1)),
            command_tx,
            command_rx: Some(command_rx),
            event_listeners: Vec::new(),
        }
    }

    /// Registers an event listener.
    ///
    /// Listeners are called synchronously in sequence order for each event.
    ///
    /// # Arguments
    ///
    /// * `listener` - Function to call for each event
    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: Fn(&SequencerEvent<T>) + Send + Sync + 'static,
    {
        self.event_listeners.push(Arc::new(listener));
    }

    /// Submits a command to the sequencer.
    ///
    /// Returns a receipt containing the assigned sequence number and result.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Errors
    ///
    /// Returns an error if the sequencer has been shut down.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use orderbook_rs::sequencer::{Sequencer, SequencerCommand};
    /// # use orderbook_rs::DefaultOrderBook;
    /// # use pricelevel::OrderId;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
    /// let command: SequencerCommand<()> = SequencerCommand::CancelOrder(OrderId::new());
    /// let receipt = sequencer.submit(command).await?;
    /// assert!(receipt.sequence_num > 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn submit(
        &self,
        command: SequencerCommand<T>,
    ) -> Result<SequencerReceipt, SequencerError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send((command, tx))
            .await
            .map_err(|_| SequencerError::Shutdown)?;
        rx.await.map_err(|_| SequencerError::Shutdown)
    }

    /// Spawns the sequencer event loop on a new task.
    ///
    /// Returns a handle that can be used to wait for shutdown.
    ///
    /// # Panics
    ///
    /// Panics if called more than once on the same Sequencer instance.
    #[must_use]
    pub fn spawn(mut self) -> SequencerHandle {
        let command_rx = self.command_rx.take().expect("spawn called twice");

        let handle = tokio::spawn(async move {
            self.run_loop(command_rx).await;
        });

        SequencerHandle { handle }
    }

    /// Runs the main event loop (single-threaded).
    ///
    /// Receives commands, assigns sequence numbers, executes on OrderBook,
    /// emits events, and sends receipts.
    async fn run_loop(
        &mut self,
        mut command_rx: mpsc::Receiver<(SequencerCommand<T>, oneshot::Sender<SequencerReceipt>)>,
    ) {
        while let Some((command, reply)) = command_rx.recv().await {
            let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
            let ts = nanos_since_epoch();

            let result = self.execute_command(&command);

            let event = SequencerEvent::new(seq, ts, command.clone(), result);

            for listener in &self.event_listeners {
                listener(&event);
            }

            let receipt = SequencerReceipt::new(seq, event.result);
            let _ = reply.send(receipt);
        }
    }

    /// Executes a command on the underlying OrderBook.
    fn execute_command(&mut self, command: &SequencerCommand<T>) -> SequencerResult {
        match command {
            SequencerCommand::AddOrder(order) => self.execute_add_order(order.clone()),
            SequencerCommand::CancelOrder(order_id) => self.execute_cancel_order(*order_id),
        }
    }

    /// Executes an add order command.
    fn execute_add_order(&mut self, order: OrderType<T>) -> SequencerResult {
        let order_id = order.id();
        match self.book.add_order(order) {
            Ok(_) => SequencerResult::OrderAdded { order_id },
            Err(e) => SequencerResult::Rejected { error: e },
        }
    }

    /// Executes a cancel order command.
    fn execute_cancel_order(&mut self, order_id: OrderId) -> SequencerResult {
        match self.book.cancel_order(order_id) {
            Ok(Some(_)) => SequencerResult::OrderCancelled { order_id },
            Ok(None) => SequencerResult::Rejected {
                error: crate::orderbook::OrderBookError::OrderNotFound(format!(
                    "order {} not found",
                    order_id
                )),
            },
            Err(e) => SequencerResult::Rejected { error: e },
        }
    }

    /// Returns a clone of the command sender.
    ///
    /// This allows creating multiple submission handles.
    #[must_use]
    pub fn sender(&self) -> mpsc::Sender<(SequencerCommand<T>, oneshot::Sender<SequencerReceipt>)> {
        self.command_tx.clone()
    }
}

/// Handle to a spawned sequencer task.
pub struct SequencerHandle {
    handle: tokio::task::JoinHandle<()>,
}

impl SequencerHandle {
    /// Waits for the sequencer to shut down.
    pub async fn wait(self) -> Result<(), tokio::task::JoinError> {
        self.handle.await
    }
}

/// Errors that can occur when interacting with the Sequencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerError {
    /// The sequencer has been shut down.
    Shutdown,
}

impl std::fmt::Display for SequencerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shutdown => write!(f, "sequencer has been shut down"),
        }
    }
}

impl std::error::Error for SequencerError {}

/// Returns the current time in nanoseconds since the Unix epoch.
#[inline]
fn nanos_since_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
