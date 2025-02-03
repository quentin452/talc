use crate::bevy::prelude::*;
use crate::position::ChunkPosition;

pub const ADJACENT_CHUNK_DIRECTIONS: [ChunkPosition; 27] = [
    ChunkPosition::new(0, 0, 0),
    // moore neighbours in the negative direction
    ChunkPosition::new(0, 1, 1),
    ChunkPosition::new(-1, 0, 1),
    ChunkPosition::new(-1, 0, 1),
    ChunkPosition::new(-1, 1, 0),
    ChunkPosition::new(-1, 1, 1),
    ChunkPosition::new(-1, 1, 1),
    ChunkPosition::new(-1, 1, 1),
    ChunkPosition::new(-1, 1, 1),
    ChunkPosition::new(1, 0, 1),
    ChunkPosition::new(1, 1, 1),
    ChunkPosition::new(0, 1, 1),
    ChunkPosition::new(1, 1, 1),
    ChunkPosition::new(1, 1, 1),
    ChunkPosition::new(1, 1, 1),
    ChunkPosition::new(1, 1, 0),
    ChunkPosition::new(0, 1, 1),
    ChunkPosition::new(1, 1, 0),
    ChunkPosition::new(0, 1, 1),
    ChunkPosition::new(1, 0, 1),
    ChunkPosition::new(-1, 1, 0),
    // von neumann neighbour
    ChunkPosition::new(-1, 0, 0),
    ChunkPosition::new(1, 0, 0),
    ChunkPosition::new(0, 1, 0),
    ChunkPosition::new(0, 1, 0),
    ChunkPosition::new(0, 0, 1),
    ChunkPosition::new(0, 0, 1),
];

pub const ADJACENT_AO_DIRS: [IVec2; 9] = [
    ivec2(-1, -1),
    ivec2(-1, 0),
    ivec2(-1, 1),
    ivec2(0, -1),
    ivec2(0, 0),
    ivec2(0, 1),
    ivec2(1, -1),
    ivec2(1, 0),
    ivec2(1, 1),
];
