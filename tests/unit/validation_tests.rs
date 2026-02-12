use orderbook_rs::OrderBook;
use pricelevel::{Hash32, OrderId, OrderType, Side, TimeInForce};

#[cfg(test)]
mod tests {
    use super::*;

    fn make_standard_order(price: u128, quantity: u64, side: Side) -> OrderType<()> {
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

    fn make_iceberg_order(price: u128, side: Side) -> OrderType<()> {
        OrderType::IcebergOrder {
            id: OrderId::new_uuid(),
            price,
            visible_quantity: 10,
            hidden_quantity: 90,
            side,
            user_id: Hash32::zero(),
            timestamp: 0,
            time_in_force: TimeInForce::Gtc,
            extra_fields: (),
        }
    }

    fn make_post_only_order(price: u128, side: Side) -> OrderType<()> {
        OrderType::PostOnly {
            id: OrderId::new_uuid(),
            price,
            quantity: 100,
            side,
            user_id: Hash32::zero(),
            timestamp: 0,
            time_in_force: TimeInForce::Gtc,
            extra_fields: (),
        }
    }

    // --- with_tick_size constructor ---

    #[test]
    fn test_with_tick_size_constructor() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        assert_eq!(book.tick_size(), Some(100));
    }

    // --- set_tick_size setter ---

    #[test]
    fn test_set_tick_size() {
        let mut book: OrderBook<()> = OrderBook::new("BTC/USD");
        assert_eq!(book.tick_size(), None);
        book.set_tick_size(50);
        assert_eq!(book.tick_size(), Some(50));
    }

    // --- Backward compatibility: no tick size ---

    #[test]
    fn test_no_tick_size_accepts_any_price() {
        let book: OrderBook<()> = OrderBook::new("BTC/USD");
        let order = make_standard_order(12345, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    // --- Valid tick sizes ---

    #[test]
    fn test_valid_tick_size_exact_multiple() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(1000, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_tick_size_larger_multiple() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(50000, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_tick_size_equals_tick() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(100, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    // --- Invalid tick sizes ---

    #[test]
    fn test_invalid_tick_size_not_multiple() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(150, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("tick size"),
            "Error message should mention tick size: {msg}"
        );
    }

    #[test]
    fn test_invalid_tick_size_off_by_one() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(1001, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_tick_size_off_by_one_below() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(999, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_err());
    }

    // --- Tick size = 1 accepts all prices ---

    #[test]
    fn test_tick_size_one_accepts_any_price() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 1);
        let order = make_standard_order(12345, 100, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    // --- Validation applies to all order types ---

    #[test]
    fn test_tick_size_rejects_iceberg_order() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_iceberg_order(150, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_err());
    }

    #[test]
    fn test_tick_size_accepts_iceberg_order_valid() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_iceberg_order(200, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tick_size_rejects_post_only_order() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_post_only_order(150, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_err());
    }

    #[test]
    fn test_tick_size_accepts_post_only_order_valid() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_post_only_order(200, Side::Buy);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    // --- Sell side validation ---

    #[test]
    fn test_tick_size_rejects_sell_order() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(150, 100, Side::Sell);
        let result = book.add_order(order);
        assert!(result.is_err());
    }

    #[test]
    fn test_tick_size_accepts_sell_order_valid() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(200, 100, Side::Sell);
        let result = book.add_order(order);
        assert!(result.is_ok());
    }

    // --- Dynamic tick size change ---

    #[test]
    fn test_set_tick_size_changes_validation() {
        let mut book: OrderBook<()> = OrderBook::new("BTC/USD");

        // No tick size — any price accepted
        let order = make_standard_order(150, 100, Side::Buy);
        assert!(book.add_order(order).is_ok());

        // Set tick size — 150 would now fail
        book.set_tick_size(100);
        let order = make_standard_order(150, 100, Side::Sell);
        assert!(book.add_order(order).is_err());

        // But 200 still works
        let order = make_standard_order(200, 100, Side::Sell);
        assert!(book.add_order(order).is_ok());
    }

    // --- add_limit_order convenience method ---

    #[test]
    fn test_add_limit_order_respects_tick_size() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let result = book.add_limit_order(
            OrderId::new_uuid(),
            150,
            100,
            Side::Buy,
            TimeInForce::Gtc,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_limit_order_valid_tick_size() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let result = book.add_limit_order(
            OrderId::new_uuid(),
            200,
            100,
            Side::Buy,
            TimeInForce::Gtc,
            None,
        );
        assert!(result.is_ok());
    }

    // --- Error display ---

    #[test]
    fn test_invalid_tick_size_error_display() {
        let book: OrderBook<()> = OrderBook::with_tick_size("BTC/USD", 100);
        let order = make_standard_order(150, 100, Side::Buy);
        let err = book.add_order(order).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("150"), "Should contain price: {msg}");
        assert!(msg.contains("100"), "Should contain tick size: {msg}");
    }
}
