use criterion::{black_box, criterion_group, criterion_main, Criterion};
use talc::{chunk::ChunkData, position::ChunkPosition};

fn bench_chunk(chunk_position: ChunkPosition) {
    let _chunk = ChunkData::generate(chunk_position);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("build chunk data", |b| {
        b.iter_with_setup(
            || {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let b = 100;
                let y = 20;
                black_box(ChunkPosition::new(
                    rng.gen_range(-b..b),
                    rng.gen_range(-y..y),
                    rng.gen_range(-b..b),
                ))
            },
            bench_chunk,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
