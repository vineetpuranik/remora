use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

async fn run_copy(payload: &[u8]) {
    // Bind a listener as the fake backend
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn backend that reads all bytes and echoes nothing
    let server = tokio::spawn(async move {
        let (mut conn, _) = listener.accept().await.unwrap();
        let (mut r, mut w) = conn.split();
        tokio::io::copy(&mut r, &mut w).await.unwrap();
    });

    // Client side: connect, write payload, shutdown
    let mut client = TcpStream::connect(addr).await.unwrap();
    client.write_all(payload).await.unwrap();
    client.shutdown().await.unwrap();

    server.await.unwrap();
}

fn bench_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("copy_bidirectional");

    for size in [1024usize, 65536, 1_048_576] {
        let payload = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &payload, |b, p| {
            b.to_async(&rt).iter(|| run_copy(p));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
