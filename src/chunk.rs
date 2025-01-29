use bevy::ecs::component::Component;
use bracket_noise::prelude::*;

use crate::{
    position::{ChunkPosition, Position, RelativePosition},
    voxel::BlockType,
};

#[derive(Component)]
pub struct Chunk;

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
pub const CHUNK_SIZE_U16: u16 = CHUNK_SIZE as u16;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_SIZE_U32: u32 = CHUNK_SIZE as u32;
pub const CHUNK_SIZE_P: usize = CHUNK_SIZE + 2;
pub const CHUNK_SIZE_P2: usize = CHUNK_SIZE_P * CHUNK_SIZE_P;
pub const CHUNK_SIZE_P3: usize = CHUNK_SIZE_P * CHUNK_SIZE_P * CHUNK_SIZE_P;
pub const CHUNK_SIZE2: usize = CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_SIZE2_I32: i32 = CHUNK_SIZE2 as i32;
pub const CHUNK_SIZE3: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_SIZE3_I32: i32 = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as i32;

/// The index of a voxel within a chunk.
/// Each chunk contains `chunk::CHUNK_SIZE3` voxels.
#[derive(Debug, Hash, Clone, Copy)]
pub struct VoxelIndex(pub usize);

impl VoxelIndex {
    /// # Panics
    /// If x, y, or z are >= `chunk::CHUNK_SIZE`
    #[must_use]
    pub const fn new(x: usize, y: usize, z: usize) -> Self {
        assert!(
            x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE,
            "Expected x, y, z to each be < chunk::CHUNK_SIZE"
        );
        Self(x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE)
    }

    #[inline]
    #[must_use]
    pub const fn i(&self) -> usize {
        self.0
    }
}

impl From<usize> for VoxelIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<VoxelIndex> for RelativePosition {
    fn from(value: VoxelIndex) -> Self {
        let x = value.i() % CHUNK_SIZE;
        let y = (value.i() / CHUNK_SIZE) % CHUNK_SIZE;
        let z = value.i() / (CHUNK_SIZE * CHUNK_SIZE);
        Self::new(x as i32, y as i32, z as i32)
    }
}

impl From<RelativePosition> for VoxelIndex {
    fn from(value: RelativePosition) -> Self {
        let x: usize = value
            .x()
            .try_into()
            .expect("[From<RelativePosition> for VoxelIndex] Expected x to be nonnegative.");
        let y: usize = value
            .y()
            .try_into()
            .expect("[From<RelativePosition> for VoxelIndex] Expected y to be nonnegative.");
        let z: usize = value
            .z()
            .try_into()
            .expect("[From<RelativePosition> for VoxelIndex] Expected z to be nonnegative.");
        Self::new(x, y, z)
    }
}

#[derive(Clone, Debug)]
enum Voxels {
    Heterogeneous(Box<[BlockType]>),
    Homogeneous(BlockType),
}

#[derive(Clone, Debug)]
pub struct ChunkData {
    voxels: Voxels,
}

impl ChunkData {
    #[inline]
    #[must_use]
    pub const fn get_block(&self, index: VoxelIndex) -> BlockType {
        match &self.voxels {
            Voxels::Homogeneous(block_type) => *block_type,
            Voxels::Heterogeneous(voxels) => voxels[index.i()],
        }
    }

    pub fn set_block(&mut self, index: VoxelIndex, block_type: BlockType) {
        match &mut self.voxels {
            Voxels::Homogeneous(old_block_type) => {
                let mut new_voxels: Box<[BlockType]> =
                    (0..CHUNK_SIZE3).map(|_| *old_block_type).collect();
                new_voxels[index.i()] = block_type;
                self.voxels = Voxels::Heterogeneous(new_voxels);
            }
            Voxels::Heterogeneous(voxels) => {
                voxels[index.i()] = block_type;

                let homogeneous = voxels.iter().all(|block| *block == block_type);
                if homogeneous {
                    self.voxels = Voxels::Homogeneous(block_type);
                }
            }
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_homogenous(&self) -> bool {
        matches!(self.voxels, Voxels::Homogeneous(_))
    }

    /// shape our voxel data based on the `chunk_pos`
    #[must_use]
    pub fn generate(chunk_position: ChunkPosition) -> Self {
        // hardcoded extremity check
        if chunk_position.y() * CHUNK_SIZE_I32 > 21 {
            return Self {
                voxels: Voxels::Homogeneous(BlockType::Air),
            };
        }
        // hardcoded extremity check
        if chunk_position.y() * CHUNK_SIZE_I32 < -53 {
            return Self {
                voxels: Voxels::Homogeneous(BlockType::Grass),
            };
        }

        let world_position = Position::from(chunk_position);
        let mut fast_noise = FastNoise::new();
        fast_noise.set_frequency(0.0254);
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;

        let voxels: Box<[BlockType; CHUNK_SIZE3]> = std::array::from_fn(|_| {
            let wx = (x + world_position.x()) as f32;
            let wy = (y + world_position.y()) as f32;
            let wz = (z + world_position.z()) as f32;

            let scale = 1.0;
            fast_noise.set_frequency(0.0254);
            let overhang = fast_noise.get_noise3d(wx * scale, wy, wz * scale) * 55.0;
            fast_noise.set_frequency(0.002591);
            let noise_2 = fast_noise.get_noise(wx + overhang, wz * scale);
            let h = noise_2 * 30.0;
            let solid = h > wy;

            let block_type = if !solid {
                BlockType::Air
            } else if (h - wy) > 1.0 {
                BlockType::Dirt
            } else {
                BlockType::Grass
            };

            x += 1;
            if x == CHUNK_SIZE_I32 {
                y += 1;
                x = 0;
                if y == CHUNK_SIZE_I32 {
                    z += 1;
                    y = 0;
                }
            }

            block_type
        })
        .into();

        if let Some(first) = voxels.first() {
            let homogeneous = voxels.iter().all(|block_type| block_type == first);
            if homogeneous {
                return Self {
                    voxels: Voxels::Homogeneous(*first),
                };
            }
        }

        Self {
            voxels: Voxels::Heterogeneous(voxels),
        }
    }
}

#[test]
fn index_functions() {
    for z in 0..CHUNK_SIZE_I32 {
        for y in 0..CHUNK_SIZE_I32 {
            for x in 0..CHUNK_SIZE_I32 {
                let pos = RelativePosition::new(x, y, z);
                let index: VoxelIndex = pos.into();
                let from_index: RelativePosition = index.into();
                assert_eq!(pos, from_index);
            }
        }
    }
}
