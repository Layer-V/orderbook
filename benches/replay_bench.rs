use criterion::{BatchSize, BenchmarkId, Criterion};
use orderbook_rs::sequencer::journal::{InMemoryJournal, Journal};
use orderbook_rs::sequencer::replay::ReplayEngine;
use orderbook_rs::sequencer::{SequencerCommand, SequencerEvent, SequencerResult};
use pricelevel::{Hash32, OrderId, OrderType, Side, TimeInForce};
use std::hint::black_box;

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

fn build_journal(n: usize) -> InMemoryJournal<()> {
    let mut journal = InMemoryJournal::with_capacity(n);
    for i in 0..n {
        let order = make_order(100 + (i % 50) as u128, 10, Side::Buy);
        let order_id = order.id();
        journal
            .append(SequencerEvent::new(
                i as u64 + 1,
                i as u64 * 1_000,
                SequencerCommand::AddOrder(order),
                SequencerResult::OrderAdded { order_id },
            ))
            .ok();
    }
    journal
}

pub fn bench_replay_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_throughput");

    for size in [1_000, 10_000, 100_000, 500_000] {
        let journal = build_journal(size);

        group.bench_with_input(BenchmarkId::new("replay_from", size), &journal, |b, j| {
            b.iter(|| {
                let (book, last_seq) =
                    ReplayEngine::replay_from(black_box(j), 0, "BTC/USD").unwrap();
                black_box((book, last_seq));
            });
        });
    }

    group.finish();
}

pub fn bench_replay_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_range");
    let journal = build_journal(100_000);

    group.bench_function("range_10k", |b| {
        b.iter(|| {
            let events = ReplayEngine::replay_range(black_box(&journal), 1, 10_000).unwrap();
            black_box(events);
        });
    });

    group.finish();
}

pub fn bench_journal_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("journal_append");

    for size in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("in_memory_append", size),
            &size,
            |b, &n| {
                b.iter_batched(
                    || {
                        let orders: Vec<OrderType<()>> = (0..n)
                            .map(|i| make_order(100 + (i % 50) as u128, 10, Side::Buy))
                            .collect();
                        (InMemoryJournal::with_capacity(n), orders)
                    },
                    |(mut journal, orders)| {
                        for (i, order) in orders.into_iter().enumerate() {
                            let order_id = order.id();
                            journal
                                .append(SequencerEvent::new(
                                    i as u64 + 1,
                                    i as u64,
                                    SequencerCommand::AddOrder(order),
                                    SequencerResult::OrderAdded { order_id },
                                ))
                                .ok();
                        }
                        black_box(journal)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

pub fn register_benchmarks(c: &mut Criterion) {
    bench_replay_throughput(c);
    bench_replay_range(c);
    bench_journal_append(c);
}
