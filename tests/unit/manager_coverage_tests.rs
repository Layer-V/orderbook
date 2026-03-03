/******************************************************************************
   Unit tests for BookManager coverage gaps.
   Covers: BookManagerStd, BookManagerTokio trait methods,
   Default impls, start_trade_processor, error paths.
******************************************************************************/

use orderbook_rs::orderbook::manager::{BookManager, BookManagerStd, BookManagerTokio};
use pricelevel::{Hash32, Id, Side, TimeInForce};

// ─── BookManagerStd ─────────────────────────────────────────────────────────

#[test]
fn std_default_creates_empty_manager() {
    let mgr: BookManagerStd<()> = BookManagerStd::default();
    assert_eq!(mgr.book_count(), 0);
    assert!(mgr.symbols().is_empty());
}

#[test]
fn std_add_and_get_book() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("BTC/USD");
    assert!(mgr.has_book("BTC/USD"));
    assert!(!mgr.has_book("ETH/USD"));
    assert_eq!(mgr.book_count(), 1);

    let symbols = mgr.symbols();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0], "BTC/USD");
}

#[test]
fn std_get_book_returns_none_for_unknown() {
    let mgr: BookManagerStd<()> = BookManagerStd::new();
    assert!(mgr.get_book("UNKNOWN").is_none());
}

#[test]
fn std_get_book_mut_returns_none_for_unknown() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    assert!(mgr.get_book_mut("UNKNOWN").is_none());
}

#[test]
fn std_get_book_returns_valid_ref() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("ETH/USD");
    let book = mgr.get_book("ETH/USD");
    assert!(book.is_some());
}

#[test]
fn std_get_book_mut_allows_modification() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("ETH/USD");
    let book = mgr.get_book_mut("ETH/USD");
    assert!(book.is_some());
}

#[test]
fn std_remove_book_returns_some_when_exists() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("BTC/USD");
    let removed = mgr.remove_book("BTC/USD");
    assert!(removed.is_some());
    assert_eq!(mgr.book_count(), 0);
    assert!(!mgr.has_book("BTC/USD"));
}

#[test]
fn std_remove_book_returns_none_when_missing() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    let removed = mgr.remove_book("MISSING");
    assert!(removed.is_none());
}

#[test]
fn std_start_trade_processor_ok_first_time() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    let result = mgr.start_trade_processor();
    assert!(result.is_ok());
}

#[test]
fn std_start_trade_processor_fails_second_time() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    let _ = mgr.start_trade_processor();
    let result = mgr.start_trade_processor();
    assert!(result.is_err());
}

#[test]
fn std_add_order_and_cancel_across_books() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("BTC/USD");
    mgr.add_book("ETH/USD");

    if let Some(book) = mgr.get_book("BTC/USD") {
        let _ = book.add_limit_order(Id::new_uuid(), 100, 10, Side::Buy, TimeInForce::Gtc, None);
    }
    if let Some(book) = mgr.get_book("ETH/USD") {
        let _ = book.add_limit_order(Id::new_uuid(), 200, 5, Side::Sell, TimeInForce::Gtc, None);
    }

    let results = mgr.cancel_all_across_books();
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("BTC/USD"));
    assert!(results.contains_key("ETH/USD"));
}

#[test]
fn std_cancel_by_user_across_books() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("BTC/USD");

    if let Some(book) = mgr.get_book("BTC/USD") {
        let _ = book.add_limit_order_with_user(
            Id::new_uuid(),
            100,
            10,
            Side::Buy,
            TimeInForce::Gtc,
            Hash32::from([42u8; 32]),
            None,
        );
    }

    let results = mgr.cancel_by_user_across_books(Hash32::from([42u8; 32]));
    assert!(results.contains_key("BTC/USD"));
}

#[test]
fn std_cancel_by_side_across_books() {
    let mut mgr: BookManagerStd<()> = BookManagerStd::new();
    mgr.add_book("BTC/USD");

    if let Some(book) = mgr.get_book("BTC/USD") {
        let _ = book.add_limit_order(Id::new_uuid(), 100, 10, Side::Buy, TimeInForce::Gtc, None);
        let _ = book.add_limit_order(Id::new_uuid(), 110, 5, Side::Sell, TimeInForce::Gtc, None);
    }

    let results = mgr.cancel_by_side_across_books(Side::Buy);
    assert!(results.contains_key("BTC/USD"));
}

// ─── BookManagerTokio ───────────────────────────────────────────────────────

#[test]
fn tokio_default_creates_empty_manager() {
    let mgr: BookManagerTokio<()> = BookManagerTokio::default();
    assert_eq!(mgr.book_count(), 0);
    assert!(mgr.symbols().is_empty());
}

#[test]
fn tokio_add_and_get_book() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    mgr.add_book("BTC/USD");
    assert!(mgr.has_book("BTC/USD"));
    assert_eq!(mgr.book_count(), 1);
}

#[test]
fn tokio_get_book_returns_none_for_unknown() {
    let mgr: BookManagerTokio<()> = BookManagerTokio::new();
    assert!(mgr.get_book("UNKNOWN").is_none());
}

#[test]
fn tokio_get_book_mut_returns_none_for_unknown() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    assert!(mgr.get_book_mut("UNKNOWN").is_none());
}

#[test]
fn tokio_remove_book_returns_some_when_exists() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    mgr.add_book("BTC/USD");
    assert!(mgr.remove_book("BTC/USD").is_some());
    assert_eq!(mgr.book_count(), 0);
}

#[test]
fn tokio_remove_book_returns_none_when_missing() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    assert!(mgr.remove_book("MISSING").is_none());
}

#[test]
fn tokio_symbols_returns_all_books() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    mgr.add_book("BTC/USD");
    mgr.add_book("ETH/USD");
    let mut symbols = mgr.symbols();
    symbols.sort();
    assert_eq!(symbols, vec!["BTC/USD", "ETH/USD"]);
}

#[test]
fn tokio_cancel_all_across_books() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    mgr.add_book("BTC/USD");
    if let Some(book) = mgr.get_book("BTC/USD") {
        let _ = book.add_limit_order(Id::new_uuid(), 100, 10, Side::Buy, TimeInForce::Gtc, None);
    }
    let results = mgr.cancel_all_across_books();
    assert!(results.contains_key("BTC/USD"));
}

#[test]
fn tokio_cancel_by_user_across_books() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    mgr.add_book("BTC/USD");
    if let Some(book) = mgr.get_book("BTC/USD") {
        let _ = book.add_limit_order_with_user(
            Id::new_uuid(),
            100,
            10,
            Side::Buy,
            TimeInForce::Gtc,
            Hash32::from([99u8; 32]),
            None,
        );
    }
    let results = mgr.cancel_by_user_across_books(Hash32::from([99u8; 32]));
    assert!(results.contains_key("BTC/USD"));
}

#[test]
fn tokio_cancel_by_side_across_books() {
    let mut mgr: BookManagerTokio<()> = BookManagerTokio::new();
    mgr.add_book("BTC/USD");
    if let Some(book) = mgr.get_book("BTC/USD") {
        let _ = book.add_limit_order(Id::new_uuid(), 100, 10, Side::Sell, TimeInForce::Gtc, None);
    }
    let results = mgr.cancel_by_side_across_books(Side::Sell);
    assert!(results.contains_key("BTC/USD"));
}
