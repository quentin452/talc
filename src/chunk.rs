use bevy::prelude::*;
use bracket_noise::prelude::*;

use crate::{
    utils::index_to_ivec3,
    voxel::BlockType,
};

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_SIZE_P: usize = CHUNK_SIZE + 2;
pub const CHUNK_SIZE_P2: usize = CHUNK_SIZE_P * CHUNK_SIZE_P;
pub const CHUNK_SIZE_P3: usize = CHUNK_SIZE_P * CHUNK_SIZE_P * CHUNK_SIZE_P;
pub const CHUNK_SIZE2: usize = CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_SIZE2_I32: i32 = CHUNK_SIZE2 as i32;
pub const CHUNK_SIZE3: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_SIZE3_I32: i32 = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as i32;

#[derive(Clone, Debug)]
enum Voxels {
    Heterogeneous(Box<[BlockType]>),
    Homogeneous(BlockType)
}

#[derive(Clone, Debug)]
pub struct ChunkData {
    voxels: Voxels,
}

impl ChunkData {
    #[inline]
    #[must_use]
    pub const fn get_block(&self, index: usize) -> BlockType {
        match &self.voxels {
            Voxels::Homogeneous(block_type) => *block_type,
            Voxels::Heterogeneous(voxels) => voxels[index]
        }
    }

    pub fn set_block(&mut self, index: usize, block_type: BlockType) {
        match &mut self.voxels {
            Voxels::Homogeneous(old_block_type) => {
                let mut new_voxels: Box<[BlockType]> = (0..CHUNK_SIZE3).map(|_| *old_block_type).collect();
                new_voxels[index] = block_type;
                self.voxels = Voxels::Heterogeneous(new_voxels);
            },
            Voxels::Heterogeneous(voxels) => {
                voxels[index] = block_type;

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
    pub fn generate(chunk_pos: IVec3) -> Self {
        // hardcoded extremity check
        if chunk_pos.y * 32 + 32 > 21 + 32 {
            return Self {
                voxels: Voxels::Homogeneous(BlockType::Air),
            };
        }
        // hardcoded extremity check
        if chunk_pos.y * 32 < -21 - 32 {
            return Self {
                voxels: Voxels::Homogeneous(BlockType::Grass),
            };
        }

        let mut voxels = vec![];
        let mut fast_noise = FastNoise::new();
        fast_noise.set_frequency(0.0254);
        for i in 0..CHUNK_SIZE3_I32 {
            let voxel_pos = (chunk_pos * 32) + index_to_ivec3(i);
            let scale = 1.0;
            fast_noise.set_frequency(0.0254);
            let overhang = fast_noise.get_noise3d(
                voxel_pos.x as f32 * scale,
                voxel_pos.y as f32,
                voxel_pos.z as f32 * scale,
            ) * 55.0;
            fast_noise.set_frequency(0.002591);
            let noise_2 =
                fast_noise.get_noise(voxel_pos.x as f32 + overhang, voxel_pos.z as f32 * scale);
            let h = noise_2 * 30.0;
            let solid = h > voxel_pos.y as f32;

            let block_type = if !solid {
                BlockType::Air
            } else if (h - voxel_pos.y as f32) > 1.0 {
                BlockType::Dirt
            } else {
                BlockType::Grass
            };
            voxels.push(block_type);
        }

        if let Some(first) = voxels.first() {
            let homogeneous = voxels.iter().all(|block_type| block_type == first);
            if homogeneous {
                return Self {
                    voxels: Voxels::Homogeneous(*first)
                }
            }
        }

        Self { voxels: Voxels::Heterogeneous(voxels.as_slice().into()) }
    }
}
