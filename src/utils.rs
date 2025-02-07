use bevy::prelude::*;

use crate::{
    chunky::chunk::CHUNK_SIZE_I32,
    position::{ChunkPosition, RelativePosition},
};

#[inline]
#[must_use]
pub const fn index_to_ivec3_bounds(i: i32, bounds: i32) -> IVec3 {
    let x = i % bounds;
    let y = (i / bounds) % bounds;
    let z = i / (bounds * bounds);
    IVec3::new(x, y, z)
}

#[inline]
#[must_use]
pub const fn index_to_ivec3_bounds_reverse(i: i32, bounds: i32) -> IVec3 {
    let z = i % bounds;
    let y = (i / bounds) % bounds;
    let x = i / (bounds * bounds);
    IVec3::new(x, y, z)
}

/// if lying on the edge of a chunk, return the edging chunk's offset.
#[inline]
#[must_use]
pub fn get_edging_chunk(pos: RelativePosition) -> Option<ChunkPosition> {
    let mut chunk_dir = IVec3::ZERO;
    if pos.x() == 0 {
        chunk_dir.x = -1;
    } else if pos.x() == CHUNK_SIZE_I32 - 1 {
        chunk_dir.x = 1;
    }
    if pos.y() == 0 {
        chunk_dir.y = -1;
    } else if pos.y() == CHUNK_SIZE_I32 - 1 {
        chunk_dir.y = 1;
    }
    if pos.z() == 0 {
        chunk_dir.z = -1;
    } else if pos.z() == CHUNK_SIZE_I32 - 1 {
        chunk_dir.z = 1;
    }
    if chunk_dir == IVec3::ZERO {
        None
    } else {
        Some(ChunkPosition(chunk_dir))
    }
}

// pos 18 bits, ao 3 bits, normal 4 bits
// 18-21-25-   left 32-25 = 7
#[inline]
#[must_use]
pub const fn make_vertex_u32(
    // position: [i32; 3], /*, normal: i32, color: Color, texture_id: u32*/
    pos: IVec3, /*, normal: i32, color: Color, texture_id: u32*/
    ao: u32,
    normal: u32,
    block_type: u32,
) -> u32 {
    pos.x as u32
        | ((pos.y as u32) << 6u32)
        | ((pos.z as u32) << 12u32)
        | (ao << 18u32)
        | (normal << 21u32)
        | (block_type << 25u32)
    // | (normal as u32) << 18u32
    // | (texture_id) << 21u32
}

/// generate a vec of indices
/// assumes vertices are made of quads, and counter clockwise ordered
#[inline]
#[must_use]
pub fn generate_indices(vertex_count: usize) -> Vec<u32> {
    let indices_count = vertex_count / 4;
    let mut indices = Vec::<u32>::with_capacity(indices_count);
    (0..indices_count).for_each(|vert_index| {
        let vert_index = vert_index as u32 * 4u32;
        indices.push(vert_index);
        indices.push(vert_index + 1);
        indices.push(vert_index + 2);
        indices.push(vert_index);
        indices.push(vert_index + 2);
        indices.push(vert_index + 3);
    });
    indices
}
