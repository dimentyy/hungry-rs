use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use hungry::crypto::{AesIgeIv, AesIgeKey, aes_ige_decrypt, aes_ige_encrypt};

fn group(c: &mut Criterion, s: &str, f: fn(&mut [u8], &AesIgeKey, &mut AesIgeIv)) {
    let mut g = c.benchmark_group(s);

    let key: AesIgeKey = [2; 32];
    let mut iv: AesIgeIv = [3; 32];

    let mut buffer = vec![4u8; 1024 * 1024 * 4];

    for size in (6..=22).map(|x| 1 << x) {
        let id = if size < 1 << 10 {
            format!("{}B", size)
        } else if size < 1 << 20 {
            format!("{}KiB", size >> 10)
        } else {
            format!("{}MiB", size >> 20)
        };

        g.throughput(Throughput::Bytes(size as u64));
        g.bench_with_input(BenchmarkId::from_parameter(id), &size, |b, _| {
            b.iter(|| {
                f(
                    black_box(&mut buffer[..size]),
                    black_box(&key),
                    black_box(&mut iv),
                );
            });
        });
    }
    g.finish();
}

fn bench(c: &mut Criterion) {
    group(c, "AES-256-IGE decryption", aes_ige_decrypt);
    group(c, "AES-256-IGE encryption", aes_ige_encrypt);
}

criterion_group!(benches, bench);
criterion_main!(benches);
