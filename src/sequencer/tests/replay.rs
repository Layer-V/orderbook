/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Tests for the deterministic replay engine.

#[cfg(test)]
mod tests {
    use crate::DefaultOrderBook;
    use crate::sequencer::journal::{InMemoryJournal, Journal};
    use crate::sequencer::replay::{ReplayEngine, ReplayError, snapshots_match};
    use crate::sequencer::{SequencerCommand, SequencerEvent, SequencerResult};
    use pricelevel::{Hash32, OrderId, OrderType, Side, TimeInForce};

    fn make_order(id: OrderId, price: u128, quantity: u64, side: Side) -> OrderType<()> {
        OrderType::Standard {
            id,
            price,
            quantity,
            side,
            user_id: Hash32::zero(),
            timestamp: 0,
            time_in_force: TimeInForce::Gtc,
            extra_fields: (),
        }
    }

    fn add_event(seq: u64, order: OrderType<()>) -> SequencerEvent<()> {
        let order_id = order.id();
        SequencerEvent::new(
            seq,
            seq * 1_000_000,
            SequencerCommand::AddOrder(order),
            SequencerResult::OrderAdded { order_id },
        )
    }

    fn cancel_event(seq: u64, order_id: OrderId) -> SequencerEvent<()> {
        SequencerEvent::new(
            seq,
            seq * 1_000_000,
            SequencerCommand::CancelOrder(order_id),
            SequencerResult::OrderCancelled { order_id },
        )
    }

    fn rejected_event(seq: u64, order_id: OrderId) -> SequencerEvent<()> {
        use crate::orderbook::OrderBookError;
        SequencerEvent::new(
            seq,
            seq * 1_000_000,
            SequencerCommand::CancelOrder(order_id),
            SequencerResult::Rejected {
                error: OrderBookError::OrderNotFound(format!("order {} not found", order_id)),
            },
        )
    }

    // -------------------------------------------------------------------------
    // Journal unit tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_journal_empty_on_creation() {
        let journal: InMemoryJournal<()> = InMemoryJournal::new();
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
        assert!(journal.last_sequence().is_none());
    }

    #[test]
    fn test_journal_append_and_len() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id = OrderId::new_uuid();
        journal
            .append(add_event(1, make_order(id, 100, 10, Side::Buy)))
            .ok();
        assert_eq!(journal.len(), 1);
        assert_eq!(journal.last_sequence(), Some(1));
    }

    #[test]
    fn test_journal_read_from_beginning() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        for i in 1..=5 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i, make_order(id, 100 + i as u128, 10, Side::Buy)))
                .ok();
        }
        let events: Vec<_> = journal.read_from(1).collect();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_journal_read_from_midpoint() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        for i in 1..=5 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i, make_order(id, 100 + i as u128, 10, Side::Buy)))
                .ok();
        }
        let events: Vec<_> = journal.read_from(3).collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence_num, 3);
    }

    #[test]
    fn test_journal_read_range() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        for i in 1..=10 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i, make_order(id, 100 + i as u128, 10, Side::Buy)))
                .ok();
        }
        let events: Vec<_> = journal.read_range(3, 6).collect();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].sequence_num, 3);
        assert_eq!(events[3].sequence_num, 6);
    }

    // -------------------------------------------------------------------------
    // ReplayEngine unit tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_replay_empty_journal_returns_error() {
        let journal: InMemoryJournal<()> = InMemoryJournal::new();
        let result = ReplayEngine::replay_from(&journal, 0, "BTC/USD");
        assert!(matches!(result, Err(ReplayError::EmptyJournal)));
    }

    #[test]
    fn test_replay_invalid_from_sequence_returns_error() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id = OrderId::new_uuid();
        journal
            .append(add_event(1, make_order(id, 100, 10, Side::Buy)))
            .ok();

        let result = ReplayEngine::replay_from(&journal, 99, "BTC/USD");
        assert!(matches!(result, Err(ReplayError::InvalidSequence { .. })));
    }

    #[test]
    fn test_replay_single_add_order() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id = OrderId::new_uuid();
        journal
            .append(add_event(1, make_order(id, 100, 10, Side::Buy)))
            .ok();

        let result = ReplayEngine::replay_from(&journal, 0, "BTC/USD");
        assert!(result.is_ok());
        let (book, last_seq) = result.unwrap();
        assert_eq!(last_seq, 1);
        let snap = book.create_snapshot(10);
        assert_eq!(snap.bids.len(), 1);
        assert_eq!(snap.asks.len(), 0);
    }

    #[test]
    fn test_replay_from_beginning_full_state() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();

        // Add 3 buy orders and 2 sell orders at different prices
        for i in 1u128..=3 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i as u64, make_order(id, 100 + i, 10, Side::Buy)))
                .ok();
        }
        for i in 4u128..=5 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i as u64, make_order(id, 200 + i, 10, Side::Sell)))
                .ok();
        }

        let (book, last_seq) = ReplayEngine::replay_from(&journal, 0, "BTC/USD").unwrap();
        assert_eq!(last_seq, 5);
        let snap = book.create_snapshot(10);
        assert_eq!(snap.bids.len(), 3);
        assert_eq!(snap.asks.len(), 2);
    }

    #[test]
    fn test_replay_from_midpoint() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        // seq 1-3: buy orders at prices 101, 102, 103
        for i in 1u128..=3 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i as u64, make_order(id, 100 + i, 10, Side::Buy)))
                .ok();
        }

        // Replay only from seq 3 — should have 1 bid level
        let (book, last_seq) = ReplayEngine::replay_from(&journal, 3, "BTC/USD").unwrap();
        assert_eq!(last_seq, 3);
        let snap = book.create_snapshot(10);
        assert_eq!(snap.bids.len(), 1);
    }

    #[test]
    fn test_replay_with_cancel_round_trip() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id = OrderId::new_uuid();

        // Add then cancel the same order
        journal
            .append(add_event(1, make_order(id, 100, 10, Side::Buy)))
            .ok();
        journal.append(cancel_event(2, id)).ok();

        let (book, last_seq) = ReplayEngine::replay_from(&journal, 0, "BTC/USD").unwrap();
        assert_eq!(last_seq, 2);
        let snap = book.create_snapshot(10);
        // Order was cancelled — book should be empty
        assert_eq!(snap.bids.len(), 0);
    }

    #[test]
    fn test_replay_skips_rejected_events() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id1 = OrderId::new_uuid();
        let id2 = OrderId::new_uuid();

        journal
            .append(add_event(1, make_order(id1, 100, 10, Side::Buy)))
            .ok();
        // seq 2: a rejected cancel — should be skipped during replay
        journal.append(rejected_event(2, id2)).ok();
        journal
            .append(add_event(3, make_order(id2, 101, 5, Side::Buy)))
            .ok();

        let (book, last_seq) = ReplayEngine::replay_from(&journal, 0, "BTC/USD").unwrap();
        assert_eq!(last_seq, 3);
        let snap = book.create_snapshot(10);
        assert_eq!(snap.bids.len(), 2);
    }

    #[test]
    fn test_replay_range_returns_correct_slice() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        for i in 1..=10u64 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i, make_order(id, 100 + i as u128, 10, Side::Buy)))
                .ok();
        }

        let events = ReplayEngine::replay_range(&journal, 4, 7).unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].sequence_num, 4);
        assert_eq!(events[3].sequence_num, 7);
    }

    #[test]
    fn test_replay_range_empty_journal() {
        let journal: InMemoryJournal<()> = InMemoryJournal::new();
        let result = ReplayEngine::<()>::replay_range(&journal, 1, 5);
        assert!(matches!(result, Err(ReplayError::EmptyJournal)));
    }

    #[test]
    fn test_replay_range_invalid_from() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id = OrderId::new_uuid();
        journal
            .append(add_event(1, make_order(id, 100, 10, Side::Buy)))
            .ok();

        let result = ReplayEngine::<()>::replay_range(&journal, 99, 200);
        assert!(matches!(result, Err(ReplayError::InvalidSequence { .. })));
    }

    #[test]
    fn test_replay_with_progress_callback() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        for i in 1..=5u64 {
            let id = OrderId::new_uuid();
            journal
                .append(add_event(i, make_order(id, 100 + i as u128, 10, Side::Buy)))
                .ok();
        }

        use std::sync::{Arc, Mutex};
        let call_count = Arc::new(Mutex::new(0u64));
        let call_count_clone = call_count.clone();
        let (_, last_seq) =
            ReplayEngine::replay_from_with_progress(&journal, 0, "BTC/USD", move |count, _seq| {
                *call_count_clone.lock().unwrap() = count;
            })
            .unwrap();

        assert_eq!(last_seq, 5);
        assert_eq!(*call_count.lock().unwrap(), 5);
    }

    // -------------------------------------------------------------------------
    // verify / snapshots_match tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_verify_matching_snapshot() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();

        // Build the expected book in parallel
        let expected_book = DefaultOrderBook::new("BTC/USD");
        for i in 1u128..=3 {
            let id = OrderId::new_uuid();
            let order = make_order(id, 100 + i, 10, Side::Buy);
            journal.append(add_event(i as u64, order)).ok();
            expected_book.add_order(order).ok();
        }

        let expected_snapshot = expected_book.create_snapshot(usize::MAX);
        let result = ReplayEngine::verify(&journal, &expected_snapshot);
        assert!(result.is_ok());
        assert!(
            result.unwrap(),
            "verify should return true for matching state"
        );
    }

    #[test]
    fn test_verify_diverged_snapshot() {
        let mut journal: InMemoryJournal<()> = InMemoryJournal::new();
        let id = OrderId::new_uuid();
        journal
            .append(add_event(1, make_order(id, 100, 10, Side::Buy)))
            .ok();

        // Build a snapshot that does NOT match (different book)
        let other_book = DefaultOrderBook::new("BTC/USD");
        let other_id = OrderId::new_uuid();
        other_book
            .add_order(make_order(other_id, 999, 50, Side::Buy))
            .ok();
        let other_snapshot = other_book.create_snapshot(usize::MAX);

        let result = ReplayEngine::verify(&journal, &other_snapshot);
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "verify should return false for diverged state"
        );
    }

    #[test]
    fn test_verify_empty_journal() {
        let journal: InMemoryJournal<()> = InMemoryJournal::new();
        let snapshot = DefaultOrderBook::new("BTC/USD").create_snapshot(usize::MAX);
        let result = ReplayEngine::verify(&journal, &snapshot);
        assert!(matches!(result, Err(ReplayError::EmptyJournal)));
    }

    #[test]
    fn test_snapshots_match_empty_books() {
        use crate::orderbook::OrderBookSnapshot;
        let a = OrderBookSnapshot {
            symbol: "BTC/USD".to_string(),
            timestamp: 0,
            bids: vec![],
            asks: vec![],
        };
        let b = OrderBookSnapshot {
            symbol: "BTC/USD".to_string(),
            timestamp: 999,
            bids: vec![],
            asks: vec![],
        };
        assert!(snapshots_match(&a, &b));
    }

    #[test]
    fn test_snapshots_match_different_symbols() {
        use crate::orderbook::OrderBookSnapshot;
        let a = OrderBookSnapshot {
            symbol: "BTC/USD".to_string(),
            timestamp: 0,
            bids: vec![],
            asks: vec![],
        };
        let b = OrderBookSnapshot {
            symbol: "ETH/USD".to_string(),
            timestamp: 0,
            bids: vec![],
            asks: vec![],
        };
        assert!(!snapshots_match(&a, &b));
    }

    // -------------------------------------------------------------------------
    // Integration: sequencer listener → journal → replay → verify
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_sequencer_journal_replay_cycle() {
        use crate::sequencer::Sequencer;
        use std::sync::{Arc, Mutex};

        let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));

        let journal = Arc::new(Mutex::new(InMemoryJournal::<()>::with_capacity(64)));
        let journal_clone = journal.clone();

        sequencer.add_listener(move |event| {
            // Clone command and result into a new owned event for the journal
            let stored = SequencerEvent::new(
                event.sequence_num,
                event.timestamp_ns,
                event.command.clone(),
                // Re-create a simplified result for journal storage
                if event.result.is_success() {
                    match &event.command {
                        SequencerCommand::AddOrder(o) => {
                            SequencerResult::OrderAdded { order_id: o.id() }
                        }
                        SequencerCommand::CancelOrder(id) => {
                            SequencerResult::OrderCancelled { order_id: *id }
                        }
                    }
                } else {
                    use crate::orderbook::OrderBookError;
                    SequencerResult::Rejected {
                        error: OrderBookError::OrderNotFound("replay-placeholder".to_string()),
                    }
                },
            );
            journal_clone.lock().unwrap().append(stored).ok();
        });

        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        // Submit 5 buy orders
        let mut submitted_ids = Vec::new();
        for i in 1u128..=5 {
            let id = OrderId::new_uuid();
            submitted_ids.push(id);
            let order = make_order(id, 100 + i, 10, Side::Buy);
            let command = SequencerCommand::AddOrder(order);
            let (tx, rx) = tokio::sync::oneshot::channel();
            sender.send((command, tx)).await.ok();
            rx.await.ok();
        }
        drop(sender);

        // Small wait for listener to flush
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let j = journal.lock().unwrap();
        assert_eq!(j.len(), 5, "journal should have 5 events");

        // Replay and verify
        let (replayed_book, last_seq) = ReplayEngine::replay_from(&*j, 0, "BTC/USD").unwrap();
        assert_eq!(last_seq, 5);
        let snap = replayed_book.create_snapshot(usize::MAX);
        assert_eq!(snap.bids.len(), 5);
        assert_eq!(snap.asks.len(), 0);
    }
}
