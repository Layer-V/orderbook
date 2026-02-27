/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Tests for sequence number ordering guarantees.

#[cfg(test)]
mod tests {
    use crate::DefaultOrderBook;
    use crate::sequencer::{Sequencer, SequencerCommand};
    use pricelevel::{Hash32, OrderId, OrderType, Side, TimeInForce};
    use std::sync::{Arc, Mutex};

    fn make_order(price: u128, quantity: u64, side: Side) -> OrderType<()> {
        OrderType::Standard {
            id: OrderId::new_uuid(),
            price,
            quantity,
            side,
            user_id: Hash32::zero(),
            timestamp: 0,
            time_in_force: TimeInForce::Gtc,
            extra_fields: (),
        }
    }

    #[tokio::test]
    async fn test_monotonic_sequence_numbers() {
        let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));

        let sequences = Arc::new(Mutex::new(Vec::new()));
        let sequences_clone = sequences.clone();

        sequencer.add_listener(move |event| {
            sequences_clone.lock().unwrap().push(event.sequence_num);
        });

        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        let mut handles = Vec::new();
        for i in 0..100 {
            let sender_clone = sender.clone();
            let handle = tokio::spawn(async move {
                let order = make_order(100 + i, 1000, Side::Buy);
                let command = SequencerCommand::AddOrder(order);
                let (tx, rx) = tokio::sync::oneshot::channel();
                sender_clone.send((command, tx)).await.ok();
                rx.await.ok()
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.ok();
        }

        drop(sender);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let seq_vec = sequences.lock().unwrap();
        assert_eq!(seq_vec.len(), 100);

        for i in 0..seq_vec.len() {
            assert_eq!(seq_vec[i], (i + 1) as u64);
        }
    }

    #[tokio::test]
    async fn test_no_gaps_in_sequence() {
        let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));

        let sequences = Arc::new(Mutex::new(Vec::new()));
        let sequences_clone = sequences.clone();

        sequencer.add_listener(move |event| {
            sequences_clone.lock().unwrap().push(event.sequence_num);
        });

        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        for i in 0..1000 {
            let order = make_order(100 + i, 1000, Side::Buy);
            let command = SequencerCommand::AddOrder(order);
            let (tx, rx) = tokio::sync::oneshot::channel();
            sender.send((command, tx)).await.ok();
            rx.await.ok();
        }

        drop(sender);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let seq_vec = sequences.lock().unwrap();
        assert_eq!(seq_vec.len(), 1000);

        for i in 0..seq_vec.len() - 1 {
            assert_eq!(seq_vec[i + 1], seq_vec[i] + 1);
        }
    }

    #[tokio::test]
    async fn test_timestamps_monotonic() {
        let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));

        let timestamps = Arc::new(Mutex::new(Vec::new()));
        let timestamps_clone = timestamps.clone();

        sequencer.add_listener(move |event| {
            timestamps_clone.lock().unwrap().push(event.timestamp_ns);
        });

        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        for i in 0..100 {
            let order = make_order(100 + i, 1000, Side::Buy);
            let command = SequencerCommand::AddOrder(order);
            let (tx, rx) = tokio::sync::oneshot::channel();
            sender.send((command, tx)).await.ok();
            rx.await.ok();
        }

        drop(sender);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let ts_vec = timestamps.lock().unwrap();

        for i in 0..ts_vec.len() - 1 {
            assert!(ts_vec[i + 1] >= ts_vec[i], "Timestamps must be monotonic");
        }
    }
}
