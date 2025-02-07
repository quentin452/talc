use std::sync::OnceLock;

use bevy::{ecs::query::QueryData, prelude::*};
use bracket_noise::prelude::*;

use crate::{
    mod_manager::prototypes::{BlockPrototype, BlockPrototypes, Prototypes},
    position::{ChunkPosition, Position, RelativePosition},
};

/// 32^3 voxels per chunk is a great compromise as it allows each vertex to be only 32 bits when sent to wgsl.
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

/// Chunks will "float up" this distance after generating.
pub const CHUNK_INITIAL_Y_OFFSET: f32 = -64.;
pub const CHUNK_FLOAT_UP_BLOCKS_PER_SECOND: f32 = 32.;

#[derive(Component)]
pub struct Chunk {
    pub position: ChunkPosition,
}

#[derive(Debug)]
pub struct ChunkData {
    pub position: ChunkPosition,
    voxels: Voxels,
}

#[derive(Clone, Debug)]
enum Voxels {
    Heterogeneous(Box<[ThinBlockPointer]>),
    Homogeneous(ThinBlockPointer),
}

impl ChunkData {
    #[inline]
    #[must_use]
    pub fn get_block(&self, index: VoxelIndex) -> &'static BlockPrototype {
        match &self.voxels {
            Voxels::Homogeneous(block_pointer) => access_block_registry(*block_pointer),
            Voxels::Heterogeneous(voxels) => access_block_registry(voxels[index.i()]),
        }
        .expect("Invalid thin block pointer.")
    }

    pub fn set_block(&mut self, index: VoxelIndex, block_type: &'static BlockPrototype) {
        match &mut self.voxels {
            Voxels::Homogeneous(old_block_type) => {
                let mut new_voxels: Box<[ThinBlockPointer]> =
                    (0..CHUNK_SIZE3).map(|_| *old_block_type).collect();
                new_voxels[index.i()] = block_type.id;
                self.voxels = Voxels::Heterogeneous(new_voxels);
            }
            Voxels::Heterogeneous(voxels) => {
                voxels[index.i()] = block_type.id;

                let homogeneous = voxels.iter().all(|&block| block == block_type.id);
                if homogeneous {
                    todo!("woo hoo");
                    //self.voxels = Voxels::Homogeneous(block_type);
                }
            }
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_homogenous(&self) -> bool {
        matches!(self.voxels, Voxels::Homogeneous(_))
    }
}

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
    pub const fn i(self) -> usize {
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

static BLOCK_REGISTRY: OnceLock<[Option<&'static BlockPrototype>; u8::MAX as usize]> =
    OnceLock::new();
type ThinBlockPointer = u16; // Classic rust reimplementing pointers. But &'static BlockPrototype is too fat :(

#[inline]
#[must_use]
pub fn access_block_registry(id: ThinBlockPointer) -> Option<&'static BlockPrototype> {
    *BLOCK_REGISTRY.get()?.get(id as usize)?
}

/// # Builds the block registry.
///
/// ## What is a block registry?
/// Each chunk stores data in a flat array of block prototypes.
/// A naive implemetation may look like `Box<[&'static BlockPrototype]>`
/// However the & borrow requires 4 bits.
/// We can reduce the memory footprint by 4x with `Box<[u16]>`
/// The block registry maps the u16 "thin pointer" back to `&'static BlockPrototype`.
///
/// # Panics
/// If the registry has already been constructed.
pub fn set_block_registry(block_prototypes: &BlockPrototypes) {
    assert!(
        BLOCK_REGISTRY.get().is_none(),
        "Block registry has already been constructed."
    );

    BLOCK_REGISTRY.get_or_init(|| {
        let mut registry = [None; u8::MAX as usize];
        for (_, &block) in block_prototypes.iter() {
            registry[block.id as usize] = Some(block);
        }
        registry
    });
}

impl ChunkData {
    /// use noise shape our voxel data based on the `chunk_pos`
    #[must_use]
    pub fn generate(block_prototypes: &BlockPrototypes, chunk_position: ChunkPosition) -> Self {
        // hardcoded extremity check
        if chunk_position.y() * CHUNK_SIZE_I32 > 285 {
            return Self {
                voxels: Voxels::Homogeneous(block_prototypes.get("air").unwrap().id),
                position: chunk_position,
            };
        }
        // hardcoded extremity check
        if chunk_position.y() * CHUNK_SIZE_I32 < -160 {
            return Self {
                voxels: Voxels::Homogeneous(block_prototypes.get("grass").unwrap().id),
                position: chunk_position,
            };
        }

        let world_position = Position::from(chunk_position);
        let mut fast_noise = FastNoise::new();
        fast_noise.set_frequency(0.0254);
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;

        let air = block_prototypes.get("air").unwrap();
        let grass = block_prototypes.get("grass").unwrap();

        let voxels: Box<[ThinBlockPointer; CHUNK_SIZE3]> = std::array::from_fn(|_| {
            let wx = (x + world_position.x()) as f32;
            let mut wy = (y + world_position.y()) as f32 - 200.;
            let wz = (z + world_position.z()) as f32;

            wy += (f32::sin(wx / 100.) + f32::cos(wz / 100.)) * 30.;

            let block_type = if wy > 0.0 {
                air
            } else {
                grass
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

            block_type.id
        })
        .into();

        if let Some(&first) = voxels.first() {
            let homogeneous = voxels.iter().all(|&block_type| block_type == first);
            if homogeneous {
                return Self {
                    voxels: Voxels::Homogeneous(first),
                    position: chunk_position,
                };
            }
        }

        Self {
            voxels: Voxels::Heterogeneous(voxels),
            position: chunk_position,
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
