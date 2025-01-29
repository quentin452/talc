use criterion::{Criterion, black_box, criterion_group, criterion_main};
use talc::{chunk::ChunkData, position::ChunkPosition};

fn bench_chunk(chunk_position: ChunkPosition) {
    let _chunk = ChunkData::generate(chunk_position);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("build chunk data", |b| {
        b.iter_with_setup(
            || {
                use rand::Rng;
                let mut rng = rand::rng();
                let b = 100;
                let y = 20;
                black_box(ChunkPosition::new(
                    rng.random_range(-b..b),
                    rng.random_range(-y..y),
                    rng.random_range(-b..b),
                ))
            },
            bench_chunk,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
