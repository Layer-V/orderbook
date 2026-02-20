//! Integration tests for mass cancel operations.

use orderbook_rs::orderbook::mass_cancel::MassCancelResult;
use orderbook_rs::{OrderBook, STPMode};
use pricelevel::{Hash32, OrderId, Side, TimeInForce};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn new_book() -> OrderBook<()> {
    OrderBook::new("TEST")
}

fn uid(byte: u8) -> Hash32 {
    Hash32::new([byte; 32])
}

// ---------------------------------------------------------------------------
// cancel_all_orders
// ---------------------------------------------------------------------------

#[test]
fn cancel_all_on_empty_book_returns_zero() {
    let book = new_book();
    let result = book.cancel_all_orders();
    assert_eq!(result.cancelled_count(), 0);
    assert!(result.cancelled_order_ids().is_empty());
    assert!(result.is_empty());
}

#[test]
fn cancel_all_removes_every_order() {
    let book = new_book();

    for price in [90, 95, 100] {
        book.add_limit_order(
            OrderId::new_uuid(),
            price,
            10,
            Side::Buy,
            TimeInForce::Gtc,
            None,
        )
        .expect("add bid");
    }
    for price in [110, 115, 120] {
        book.add_limit_order(
            OrderId::new_uuid(),
            price,
            10,
            Side::Sell,
            TimeInForce::Gtc,
            None,
        )
        .expect("add ask");
    }

    let result = book.cancel_all_orders();

    assert_eq!(result.cancelled_count(), 6);
    assert_eq!(result.cancelled_order_ids().len(), 6);
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
}

#[test]
fn cancel_all_cleans_order_locations() {
    let book = new_book();
    let id = OrderId::new_uuid();
    book.add_limit_order(id, 100, 10, Side::Buy, TimeInForce::Gtc, None)
        .expect("add");

    let _ = book.cancel_all_orders();
    assert_eq!(book.best_bid(), None);
}

// ---------------------------------------------------------------------------
// cancel_orders_by_side
// ---------------------------------------------------------------------------

#[test]
fn cancel_by_side_buy_leaves_asks() {
    let book = new_book();

    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("bid");
    book.add_limit_order(
        OrderId::new_uuid(),
        95,
        5,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("bid 2");
    book.add_limit_order(
        OrderId::new_uuid(),
        200,
        8,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("ask");

    let result = book.cancel_orders_by_side(Side::Buy);

    assert_eq!(result.cancelled_count(), 2);
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), Some(200));
}

#[test]
fn cancel_by_side_sell_leaves_bids() {
    let book = new_book();

    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("bid");
    book.add_limit_order(
        OrderId::new_uuid(),
        200,
        8,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("ask");
    book.add_limit_order(
        OrderId::new_uuid(),
        210,
        3,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("ask 2");

    let result = book.cancel_orders_by_side(Side::Sell);

    assert_eq!(result.cancelled_count(), 2);
    assert_eq!(book.best_bid(), Some(100));
    assert_eq!(book.best_ask(), None);
}

#[test]
fn cancel_by_side_on_empty_side() {
    let book = new_book();
    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("bid");

    let result = book.cancel_orders_by_side(Side::Sell);
    assert!(result.is_empty());
    assert_eq!(book.best_bid(), Some(100));
}

// ---------------------------------------------------------------------------
// cancel_orders_by_user
// ---------------------------------------------------------------------------

#[test]
fn cancel_by_user_removes_only_matching_orders() {
    let book = new_book();
    let user_a = uid(1);
    let user_b = uid(2);

    let id_a1 = OrderId::new_uuid();
    let id_a2 = OrderId::new_uuid();
    let id_b1 = OrderId::new_uuid();

    book.add_limit_order_with_user(id_a1, 100, 10, Side::Buy, TimeInForce::Gtc, user_a, None)
        .expect("a1");
    book.add_limit_order_with_user(id_a2, 200, 5, Side::Sell, TimeInForce::Gtc, user_a, None)
        .expect("a2");
    book.add_limit_order_with_user(id_b1, 95, 20, Side::Buy, TimeInForce::Gtc, user_b, None)
        .expect("b1");

    let result = book.cancel_orders_by_user(user_a);
    assert_eq!(result.cancelled_count(), 2);
    assert!(result.cancelled_order_ids().contains(&id_a1));
    assert!(result.cancelled_order_ids().contains(&id_a2));

    // user_b order remains
    assert_eq!(book.best_bid(), Some(95));
    assert_eq!(book.best_ask(), None);
}

#[test]
fn cancel_by_user_no_match_returns_zero() {
    let book = new_book();
    let user_a = uid(1);
    let user_b = uid(2);

    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        user_a,
        None,
    )
    .expect("a1");

    let result = book.cancel_orders_by_user(user_b);
    assert!(result.is_empty());
    assert_eq!(book.best_bid(), Some(100));
}

#[test]
fn cancel_by_user_across_multiple_levels_and_sides() {
    let book = new_book();
    let user = uid(1);
    let other = uid(2);

    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        user,
        None,
    )
    .expect("buy 100");
    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        95,
        5,
        Side::Buy,
        TimeInForce::Gtc,
        user,
        None,
    )
    .expect("buy 95");
    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        200,
        8,
        Side::Sell,
        TimeInForce::Gtc,
        user,
        None,
    )
    .expect("sell 200");
    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        90,
        20,
        Side::Buy,
        TimeInForce::Gtc,
        other,
        None,
    )
    .expect("other buy");

    let result = book.cancel_orders_by_user(user);
    assert_eq!(result.cancelled_count(), 3);
    assert_eq!(book.best_bid(), Some(90));
}

// ---------------------------------------------------------------------------
// cancel_orders_by_price_range
// ---------------------------------------------------------------------------

#[test]
fn cancel_by_price_range_inclusive_boundaries() {
    let book = new_book();

    let id1 = OrderId::new_uuid();
    let id2 = OrderId::new_uuid();
    let id3 = OrderId::new_uuid();

    book.add_limit_order(id1, 100, 10, Side::Buy, TimeInForce::Gtc, None)
        .expect("100");
    book.add_limit_order(id2, 200, 10, Side::Buy, TimeInForce::Gtc, None)
        .expect("200");
    book.add_limit_order(id3, 300, 10, Side::Buy, TimeInForce::Gtc, None)
        .expect("300");

    let result = book.cancel_orders_by_price_range(Side::Buy, 100, 200);
    assert_eq!(result.cancelled_count(), 2);
    assert!(result.cancelled_order_ids().contains(&id1));
    assert!(result.cancelled_order_ids().contains(&id2));
    assert_eq!(book.best_bid(), Some(300));
}

#[test]
fn cancel_by_price_range_single_price() {
    let book = new_book();

    let id = OrderId::new_uuid();
    book.add_limit_order(id, 150, 10, Side::Sell, TimeInForce::Gtc, None)
        .expect("add");
    book.add_limit_order(
        OrderId::new_uuid(),
        200,
        10,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("add 2");

    let result = book.cancel_orders_by_price_range(Side::Sell, 150, 150);
    assert_eq!(result.cancelled_count(), 1);
    assert!(result.cancelled_order_ids().contains(&id));
    assert_eq!(book.best_ask(), Some(200));
}

#[test]
fn cancel_by_price_range_inverted_returns_zero() {
    let book = new_book();
    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("add");

    let result = book.cancel_orders_by_price_range(Side::Buy, 200, 100);
    assert!(result.is_empty());
    assert_eq!(book.best_bid(), Some(100));
}

#[test]
fn cancel_by_price_range_no_orders_in_range() {
    let book = new_book();
    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("add");

    let result = book.cancel_orders_by_price_range(Side::Buy, 200, 300);
    assert!(result.is_empty());
}

#[test]
fn cancel_by_price_range_multiple_orders_at_same_level() {
    let book = new_book();

    let id1 = OrderId::new_uuid();
    let id2 = OrderId::new_uuid();

    book.add_limit_order(id1, 100, 10, Side::Buy, TimeInForce::Gtc, None)
        .expect("add 1");
    book.add_limit_order(id2, 100, 20, Side::Buy, TimeInForce::Gtc, None)
        .expect("add 2");

    let result = book.cancel_orders_by_price_range(Side::Buy, 100, 100);
    assert_eq!(result.cancelled_count(), 2);
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
}

#[test]
fn cancel_by_price_range_on_wrong_side_returns_zero() {
    let book = new_book();
    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("add bid");

    // Asks side has nothing at 100
    let result = book.cancel_orders_by_price_range(Side::Sell, 100, 100);
    assert!(result.is_empty());
    assert_eq!(book.best_bid(), Some(100));
}

// ---------------------------------------------------------------------------
// Mixed order types
// ---------------------------------------------------------------------------

#[test]
fn cancel_all_with_iceberg_orders() {
    let book = new_book();

    book.add_iceberg_order(
        OrderId::new_uuid(),
        100,
        5,
        15,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("iceberg");
    book.add_limit_order(
        OrderId::new_uuid(),
        200,
        10,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("limit");

    let result = book.cancel_all_orders();
    assert_eq!(result.cancelled_count(), 2);
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
}

#[test]
fn cancel_all_with_post_only_orders() {
    let book = new_book();

    book.add_post_only_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("post-only");
    book.add_limit_order(
        OrderId::new_uuid(),
        200,
        10,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("limit");

    let result = book.cancel_all_orders();
    assert_eq!(result.cancelled_count(), 2);
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
}

// ---------------------------------------------------------------------------
// MassCancelResult struct
// ---------------------------------------------------------------------------

#[test]
fn mass_cancel_result_default_is_empty() {
    let result = MassCancelResult::default();
    assert!(result.is_empty());
    assert_eq!(result.cancelled_count(), 0);
    assert!(result.cancelled_order_ids().is_empty());
}

#[test]
fn mass_cancel_result_display() {
    let result = MassCancelResult::default();
    let display = format!("{result}");
    assert!(display.contains("0"));
}

// ---------------------------------------------------------------------------
// STP-enabled book
// ---------------------------------------------------------------------------

#[test]
fn cancel_by_user_on_stp_enabled_book() {
    let mut book: OrderBook<()> = OrderBook::new("TEST");
    book.set_stp_mode(STPMode::CancelTaker);

    let user_a = uid(1);
    let user_b = uid(2);

    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        user_a,
        None,
    )
    .expect("a buy");
    book.add_limit_order_with_user(
        OrderId::new_uuid(),
        200,
        5,
        Side::Sell,
        TimeInForce::Gtc,
        user_b,
        None,
    )
    .expect("b sell");

    let result = book.cancel_orders_by_user(user_a);
    assert_eq!(result.cancelled_count(), 1);
    assert_eq!(book.best_ask(), Some(200));
}

// ---------------------------------------------------------------------------
// Sequential mass cancels
// ---------------------------------------------------------------------------

#[test]
fn double_cancel_all_is_idempotent() {
    let book = new_book();

    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("add");

    let r1 = book.cancel_all_orders();
    assert_eq!(r1.cancelled_count(), 1);

    let r2 = book.cancel_all_orders();
    assert!(r2.is_empty());
}

#[test]
fn cancel_by_side_then_cancel_all() {
    let book = new_book();

    book.add_limit_order(
        OrderId::new_uuid(),
        100,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None,
    )
    .expect("bid");
    book.add_limit_order(
        OrderId::new_uuid(),
        200,
        5,
        Side::Sell,
        TimeInForce::Gtc,
        None,
    )
    .expect("ask");

    let r1 = book.cancel_orders_by_side(Side::Buy);
    assert_eq!(r1.cancelled_count(), 1);

    let r2 = book.cancel_all_orders();
    assert_eq!(r2.cancelled_count(), 1); // only the ask remains
}
