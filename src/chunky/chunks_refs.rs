use std::{hash::Hash, sync::Arc};

use crate::bevy::prelude::*;

use crate::{
    mod_manager::prototypes::BlockPrototype,
    position::{ChunkPosition, RelativePosition},
    utils::index_to_ivec3_bounds,
};

use super::{
    async_chunkloader::Chunks,
    chunk::{CHUNK_SIZE, CHUNK_SIZE_I32, ChunkData, VoxelIndex},
    quad::Direction,
};

// Pointers to chunk data, repersented as the middle one with all their neighbours in 3x3x3 cube.
#[derive(Clone)]
pub struct ChunkRefs {
    pub adjacent_chunks: [Arc<ChunkData>; 27],
    pub center_chunk_position: ChunkPosition,
}

impl ChunkRefs {
    /// construct a `ChunkRefs` at `middle_chunk` position
    /// # Panics
    /// if `ChunkData` doesn't exist in input `world_data`
    #[must_use]
    pub fn try_new(chunks: &Chunks, center_chunk_position: ChunkPosition) -> Option<Self> {
        let get_chunk = |i| {
            //let offset = ADJACENT_CHUNK_DIRECTIONS[i] + IVec3::NEG_ONE;
            let offset = ChunkPosition(index_to_ivec3_bounds(i, 3) + IVec3::NEG_ONE);
            chunks.0.get(&(center_chunk_position + offset))
        };
        #[rustfmt::skip]
        let adjacent_chunks: [Arc<ChunkData>; 27] = [
          get_chunk(0)?.clone(), get_chunk(1)?.clone(), get_chunk(2)?.clone(),
          get_chunk(3)?.clone(), get_chunk(4)?.clone(), get_chunk(5)?.clone(),
          get_chunk(6)?.clone(), get_chunk(7)?.clone(), get_chunk(8)?.clone(),

          get_chunk(9)?.clone(), get_chunk(10)?.clone(), get_chunk(11)?.clone(),
          get_chunk(12)?.clone(), get_chunk(13)?.clone(), get_chunk(14)?.clone(),
          get_chunk(15)?.clone(), get_chunk(16)?.clone(), get_chunk(17)?.clone(),

          get_chunk(18)?.clone(), get_chunk(19)?.clone(), get_chunk(20)?.clone(),
          get_chunk(21)?.clone(), get_chunk(22)?.clone(), get_chunk(23)?.clone(),
          get_chunk(24)?.clone(), get_chunk(25)?.clone(), get_chunk(26)?.clone(),
        ];
        Some(Self {
            adjacent_chunks,
            center_chunk_position,
        })
    }

    #[must_use]
    pub fn is_all_voxels_same(&self) -> bool {
        let block_type = if self.adjacent_chunks[0].is_homogenous() {
            self.adjacent_chunks[0].get_block(0.into())
        } else {
            return false;
        };
        self.adjacent_chunks
            .iter()
            .all(|chunk| chunk.is_homogenous() && chunk.get_block(0.into()) == block_type)
    }

    /// helper function to get block data that may exceed the bounds of the middle chunk
    /// input position is local pos to middle chunk
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn get_block(&self, pos: RelativePosition) -> &'static BlockPrototype {
        let x = (pos.x() + CHUNK_SIZE_I32) as usize;
        let y = (pos.y() + CHUNK_SIZE_I32) as usize;
        let z = (pos.z() + CHUNK_SIZE_I32) as usize;
        let (x_chunk, x) = ((x / CHUNK_SIZE) as i32, (x % CHUNK_SIZE));
        let (y_chunk, y) = ((y / CHUNK_SIZE) as i32, (y % CHUNK_SIZE));
        let (z_chunk, z) = ((z / CHUNK_SIZE) as i32, (z % CHUNK_SIZE));

        let chunk_index = Self::vec3_to_chunk_index(IVec3::new(x_chunk, y_chunk, z_chunk));
        let chunk_data = &self.adjacent_chunks[chunk_index];
        let i = VoxelIndex::new(x, y, z);

        chunk_data.get_block(i)
    }

    /// helper function to get voxels
    /// panics if the local pos is outside the middle chunk
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn get_block_no_neighbour(&self, pos: RelativePosition) -> &'static BlockPrototype {
        let chunk_data: &Arc<ChunkData> = &self.adjacent_chunks[13];
        chunk_data.get_block(pos.into())
    }

    /// helper function to sample adjacent(back,left,down) voxels
    #[must_use]
    pub fn get_adjacent_blocks(
        &self,
        pos: RelativePosition,
        // current back, left, down
    ) -> (
        &'static BlockPrototype,
        &'static BlockPrototype,
        &'static BlockPrototype,
        &'static BlockPrototype,
    ) {
        let current = self.get_block(pos);
        let back = self.get_block(pos + RelativePosition::new(0, 0, -1));
        let left = self.get_block(pos + RelativePosition::new(-1, 0, 0));
        let down = self.get_block(pos + RelativePosition::new(0, -1, 0));
        (current, back, left, down)
    }

    /// helper function to sample adjacent voxels, von neuman include all facing planes
    #[must_use]
    pub fn get_von_neumann(
        &self,
        pos: RelativePosition,
    ) -> [(Direction, &'static BlockPrototype); 6] {
        [
            (
                Direction::Back,
                self.get_block(pos + RelativePosition::new(0, 0, -1)),
            ),
            (
                Direction::Forward,
                self.get_block(pos + RelativePosition::new(0, 0, 1)),
            ),
            (
                Direction::Down,
                self.get_block(pos + RelativePosition::new(0, -1, 0)),
            ),
            (
                Direction::Up,
                self.get_block(pos + RelativePosition::new(0, 1, 0)),
            ),
            (
                Direction::Left,
                self.get_block(pos + RelativePosition::new(-1, 0, 0)),
            ),
            (
                Direction::Right,
                self.get_block(pos + RelativePosition::new(1, 0, 0)),
            ),
        ]
    }

    #[must_use]
    pub const fn vec3_to_chunk_index(vec: IVec3) -> usize {
        let x_i = vec.x % 3;
        let y_i = vec.y * 3;
        let z_i = vec.z * (3 * 3);
        (x_i + y_i + z_i) as usize
    }

    #[must_use]
    pub fn get_2(
        &self,
        pos: RelativePosition,
        offset: RelativePosition,
    ) -> (&'static BlockPrototype, &'static BlockPrototype) {
        let first = self.get_block(pos);
        let second = self.get_block(pos + offset);
        (first, second)
    }
}

impl Hash for ChunkRefs {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.center_chunk_position.hash(state);
    }
}

impl PartialEq<ChunkPosition> for ChunkRefs {
    fn eq(&self, other: &ChunkPosition) -> bool {
        *other == self.center_chunk_position
    }
}
