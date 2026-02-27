/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Tests for concurrent command submission.

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
    async fn test_concurrent_submissions() {
        let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));

        let sequences = Arc::new(Mutex::new(Vec::new()));
        let sequences_clone = sequences.clone();

        sequencer.add_listener(move |event| {
            sequences_clone.lock().unwrap().push(event.sequence_num);
        });

        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        let mut handles = Vec::new();
        for i in 0..10 {
            let sender_clone = sender.clone();
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    let order = make_order(100 + i * 10 + j, 1000, Side::Buy);
                    let command = SequencerCommand::AddOrder(order);
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    sender_clone.send((command, tx)).await.ok();
                    rx.await.ok();
                }
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
    async fn test_multiple_listeners() {
        let mut sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));

        let count1 = Arc::new(Mutex::new(0));
        let count2 = Arc::new(Mutex::new(0));

        let count1_clone = count1.clone();
        let count2_clone = count2.clone();

        sequencer.add_listener(move |_event| {
            *count1_clone.lock().unwrap() += 1;
        });

        sequencer.add_listener(move |_event| {
            *count2_clone.lock().unwrap() += 1;
        });

        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        for i in 0..50 {
            let order = make_order(100 + i, 1000, Side::Buy);
            let command = SequencerCommand::AddOrder(order);
            let (tx, rx) = tokio::sync::oneshot::channel();
            sender.send((command, tx)).await.ok();
            rx.await.ok();
        }

        drop(sender);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_eq!(*count1.lock().unwrap(), 50);
        assert_eq!(*count2.lock().unwrap(), 50);
    }
}
