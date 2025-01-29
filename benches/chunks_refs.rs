use criterion::{Criterion, black_box, criterion_group, criterion_main};
use talc::{
    chunk::{CHUNK_SIZE, CHUNK_SIZE_I32, CHUNK_SIZE_P, CHUNK_SIZE3_I32, VoxelIndex},
    chunks_refs::ChunksRefs,
    position::RelativePosition,
    voxel::BlockType,
};

fn iter_chunkrefs_padding(chunks_refs: &ChunksRefs) {
    for x in -1..CHUNK_SIZE_P as i32 {
        for z in -1..CHUNK_SIZE_P as i32 {
            for y in -1..CHUNK_SIZE_P as i32 {
                let pos = RelativePosition::new(x, y, z);
                let _b = chunks_refs.get_block(pos);
            }
        }
    }
}

fn iter_chunkrefs(chunks_refs: &ChunksRefs) {
    for x in 0..CHUNK_SIZE_I32 {
        for z in 0..CHUNK_SIZE_I32 {
            for y in 0..CHUNK_SIZE_I32 {
                let pos = RelativePosition::new(x, y, z);
                let _b = chunks_refs.get_block(pos);
            }
        }
    }
}

fn iter_vec(data: &[BlockType]) {
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let position = RelativePosition::new(x as i32, y as i32, z as i32);
                let index: VoxelIndex = position.into();
                let _b = black_box(data[index.0]);
            }
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("iter chunk_refs ", |b| {
        b.iter_with_setup(
            || ChunksRefs::make_dummy_chunk_refs(0),
            |i| iter_chunkrefs(&i),
        );
    });
    c.bench_function("iter chunk_refs padding ", |b| {
        b.iter_with_setup(
            || ChunksRefs::make_dummy_chunk_refs(0),
            |i| iter_chunkrefs_padding(&i),
        );
    });
    c.bench_function("iter vec", |b| {
        b.iter_with_setup(
            || {
                let mut d = vec![];
                for _ in 0..CHUNK_SIZE3_I32 {
                    d.push(BlockType::Air);
                }
                d
            },
            |i| iter_vec(&i),
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
