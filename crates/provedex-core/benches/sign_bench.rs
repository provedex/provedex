use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use provedex_core::{
    canonical_json, compute_self_hash, AgentEvent, Ledger, LedgerSession, SignedEvent,
    SigningKeypair, GENESIS_PARENT_HASH,
};

fn fixture_event() -> AgentEvent {
    AgentEvent::ModelInvoked {
        model_id: "gpt-4o".into(),
        prompt_sha256: "9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567".into(),
        response_sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".into(),
        prompt_tokens: 482,
        response_tokens: 71,
    }
}

fn bench_canonical_json(c: &mut Criterion) {
    let event = fixture_event();
    let value = serde_json::to_value(&event).unwrap();
    let mut group = c.benchmark_group("canonical_json");
    group.throughput(Throughput::Elements(1));
    group.bench_function("ModelInvoked", |b| {
        b.iter(|| {
            let _ = canonical_json(black_box(&value));
        });
    });
    group.finish();
}

fn bench_compute_self_hash(c: &mut Criterion) {
    let event = fixture_event();
    let mut group = c.benchmark_group("compute_self_hash");
    group.throughput(Throughput::Elements(1));
    group.bench_function("ModelInvoked", |b| {
        b.iter(|| {
            let _ = compute_self_hash(
                black_box(0),
                black_box(1_700_000_000_000_000_000),
                black_box(&event),
                black_box(GENESIS_PARENT_HASH),
            )
            .unwrap();
        });
    });
    group.finish();
}

fn bench_seal_only(c: &mut Criterion) {
    let kp = SigningKeypair::generate();
    let mut group = c.benchmark_group("seal_only");
    group.throughput(Throughput::Elements(1));
    group.bench_function("ModelInvoked", |b| {
        let mut parent = GENESIS_PARENT_HASH.to_string();
        let mut seq = 0u64;
        b.iter(|| {
            let signed = SignedEvent::seal(
                black_box(seq),
                black_box(fixture_event()),
                black_box(&parent),
                black_box(&kp),
            )
            .unwrap();
            parent = signed.self_hash;
            seq += 1;
        });
    });
    group.finish();
}

fn bench_seal_and_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("seal_and_append");
    group.throughput(Throughput::Elements(1));
    // Append fsyncs the file. Smaller sample size keeps the bench under a minute.
    group.sample_size(50);
    group.bench_function("with_fsync", |b| {
        let dir = tempfile::tempdir().unwrap();
        let kp = SigningKeypair::generate();
        let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
        let session = LedgerSession::open(kp, ledger, "bench".into()).unwrap();
        b.iter(|| {
            let _ = session.seal_and_append(black_box(fixture_event())).unwrap();
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_canonical_json,
    bench_compute_self_hash,
    bench_seal_only,
    bench_seal_and_append
);
criterion_main!(benches);
