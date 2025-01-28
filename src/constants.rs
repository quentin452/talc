use bevy::{
    math::{IVec2, ivec2},
    prelude::IVec3,
};

pub const ADJACENT_CHUNK_DIRECTIONS: [IVec3; 27] = [
    IVec3 { x: 0, y: 0, z: 0 },
    // moore neighbours in the negative direction
    IVec3 { x: 0, y: -1, z: -1 },
    IVec3 { x: -1, y: 0, z: -1 },
    IVec3 { x: -1, y: 0, z: 1 },
    IVec3 { x: -1, y: -1, z: 0 },
    IVec3 {
        x: -1,
        y: -1,
        z: -1,
    },
    IVec3 { x: -1, y: 1, z: -1 },
    IVec3 { x: -1, y: -1, z: 1 },
    IVec3 { x: -1, y: 1, z: 1 },
    IVec3 { x: 1, y: 0, z: -1 },
    IVec3 { x: 1, y: -1, z: -1 },
    IVec3 { x: 0, y: 1, z: -1 },
    IVec3 { x: 1, y: 1, z: 1 },
    IVec3 { x: 1, y: -1, z: 1 },
    IVec3 { x: 1, y: 1, z: -1 },
    IVec3 { x: 1, y: 1, z: 0 },
    IVec3 { x: 0, y: 1, z: 1 },
    IVec3 { x: 1, y: -1, z: 0 },
    IVec3 { x: 0, y: -1, z: 1 },
    IVec3 { x: 1, y: 0, z: 1 },
    IVec3 { x: -1, y: 1, z: 0 },
    // von neumann neighbour
    IVec3 { x: -1, y: 0, z: 0 },
    IVec3 { x: 1, y: 0, z: 0 },
    IVec3 { x: 0, y: -1, z: 0 },
    IVec3 { x: 0, y: 1, z: 0 },
    IVec3 { x: 0, y: 0, z: -1 },
    IVec3 { x: 0, y: 0, z: 1 },
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
