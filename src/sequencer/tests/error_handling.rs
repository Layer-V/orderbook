/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 27/2/26
******************************************************************************/

//! Tests for error handling in the Sequencer.

#[cfg(test)]
mod tests {
    use crate::DefaultOrderBook;
    use crate::sequencer::{Sequencer, SequencerCommand, SequencerResult};
    use pricelevel::{Hash32, OrderId, OrderType, Side, TimeInForce};

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
    async fn test_invalid_cancel_rejected() {
        let sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        let command = SequencerCommand::CancelOrder(OrderId::new());
        let (tx, rx) = tokio::sync::oneshot::channel();
        sender.send((command, tx)).await.ok();

        let receipt = rx.await.ok();
        assert!(receipt.is_some());

        let receipt = receipt.unwrap();
        assert!(receipt.result.is_rejected());

        drop(sender);
    }

    #[tokio::test]
    async fn test_sequence_continues_after_error() {
        let sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        let command1 = SequencerCommand::CancelOrder(OrderId::new());
        let (tx1, rx1) = tokio::sync::oneshot::channel();
        sender.send((command1, tx1)).await.ok();
        let receipt1 = rx1.await.ok().unwrap();
        assert!(receipt1.result.is_rejected());
        assert_eq!(receipt1.sequence_num, 1);

        let order = make_order(100, 1000, Side::Buy);
        let command2 = SequencerCommand::AddOrder(order);
        let (tx2, rx2) = tokio::sync::oneshot::channel();
        sender.send((command2, tx2)).await.ok();
        let receipt2 = rx2.await.ok().unwrap();
        assert!(receipt2.result.is_success());
        assert_eq!(receipt2.sequence_num, 2);

        drop(sender);
    }

    #[tokio::test]
    async fn test_receipt_success_flag() {
        let sequencer = Sequencer::<()>::new(DefaultOrderBook::new("BTC/USD"));
        let sender = sequencer.sender();
        let _handle = sequencer.spawn();

        let order = make_order(100, 1000, Side::Buy);
        let command = SequencerCommand::AddOrder(order);
        let (tx, rx) = tokio::sync::oneshot::channel();
        sender.send((command, tx)).await.ok();

        let receipt = rx.await.ok().unwrap();
        assert!(receipt.is_success());
        assert!(matches!(receipt.result, SequencerResult::OrderAdded { .. }));

        drop(sender);
    }
}
