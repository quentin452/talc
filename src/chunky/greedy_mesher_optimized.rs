use bevy::{platform_support::collections::HashMap, prelude::*};

use crate::{
    mod_manager::prototypes::BlockPrototype,
    position::Position,
    render::chunk_material::{ChunkMaterial, PackedQuad},
};

use super::{
    chunk::{CHUNK_SIZE, CHUNK_SIZE_P, CHUNK_SIZE3},
    chunks_refs::ChunkRefs,
    constants::ADJACENT_AO_DIRS,
    face_direction::FaceDir,
    lod::Lod,
};

#[inline]
fn add_voxel_to_axis_cols(
    block: &'static BlockPrototype,
    x: usize,
    y: usize,
    z: usize,
    axis_cols: &mut [[[u64; CHUNK_SIZE_P]; CHUNK_SIZE_P]; 3],
) {
    if !block.is_transparent {
        // x,z - y axis
        axis_cols[0][z][x] |= 1u64 << y as u64;
        // z,y - x axis
        axis_cols[1][y][z] |= 1u64 << x as u64;
        // x,y - z axis
        axis_cols[2][y][x] |= 1u64 << z as u64;
    }
}

fn calculate_ao(
    chunks_refs: &ChunkRefs,
    axis_cols: &[[[u64; 34]; 34]; 3],
) -> [HashMap<u32, HashMap<u32, [u32; CHUNK_SIZE]>>; 6] {
    // the cull mask to perform greedy slicing, based on solids on previous axis_cols
    #[allow(clippy::large_stack_arrays)]
    let mut col_face_masks = [[[0u64; CHUNK_SIZE_P]; CHUNK_SIZE_P]; 6];

    // face culling
    for axis in 0..=2 {
        for z in 0..CHUNK_SIZE_P {
            for x in 0..CHUNK_SIZE_P {
                // set if current is solid, and next is air
                let col = axis_cols[axis][z][x];

                // sample descending axis, and set true when air meets solid
                col_face_masks[2 * axis][z][x] = col & !(col << 1);
                // sample ascending axis, and set true when air meets solid
                col_face_masks[2 * axis + 1][z][x] = col & !(col >> 1);
            }
        }
    }

    // greedy meshing planes for every axis (6)
    // key(block + ao) -> HashMap<axis(0-32), binary_plane>
    // note(leddoo): don't ask me how this isn't a massive blottleneck.
    //  might become an issue in the future, when there are more block types.
    //  consider using a single hashmap with key (axis, block_hash, y).
    let mut data: [HashMap<u32, HashMap<u32, [u32; CHUNK_SIZE]>>; 6] = [
        HashMap::default(),
        HashMap::default(),
        HashMap::default(),
        HashMap::default(),
        HashMap::default(),
        HashMap::default(),
    ];

    // find faces and build binary planes based on the voxel block+ao etc...
    for axis in 0..6 {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                // skip padded by adding 1(for x padding) and (z+1) for (z padding)
                let mut col = col_face_masks[axis][z + 1][x + 1];

                // removes the right most padding value, because it's invalid
                col >>= 1;
                // removes the left most padding value, because it's invalid
                col &= !(1 << CHUNK_SIZE as u64);

                while col != 0 {
                    let y = col.trailing_zeros();
                    // clear least significant set bit
                    col &= col - 1;

                    // get the voxel position based on axis
                    let voxel_pos = match axis {
                        0 | 1 => Position::new(x as i32, y as i32, z as i32), // down,up
                        2 | 3 => Position::new(y as i32, z as i32, x as i32), // left, right
                        _ => Position::new(x as i32, z as i32, y as i32),     // forward, back
                    };

                    // calculate ambient occlusion
                    let mut ao_index = 0;
                    for (ao_i, ao_offset) in ADJACENT_AO_DIRS.iter().enumerate() {
                        // ambient occlusion is sampled based on axis(ascent or descent)
                        let ao_sample_offset = match axis {
                            0 => Position::new(ao_offset.x, -1, ao_offset.y), // down
                            1 => Position::new(ao_offset.x, 1, ao_offset.y),  // up
                            2 => Position::new(-1, ao_offset.y, ao_offset.x), // left
                            3 => Position::new(1, ao_offset.y, ao_offset.x),  // right
                            4 => Position::new(ao_offset.x, ao_offset.y, -1), // forward
                            _ => Position::new(ao_offset.x, ao_offset.y, 1),  // back
                        };
                        let ao_voxel_pos = voxel_pos + ao_sample_offset;
                        let ao_block = chunks_refs.get_block(ao_voxel_pos);
                        if !ao_block.is_transparent {
                            ao_index |= 1u32 << ao_i;
                        }
                    }

                    let current_voxel = chunks_refs.get_block_no_neighbour(voxel_pos);
                    // let current_voxel = chunks_refs.get_block(voxel_pos);
                    // we can only greedy mesh same block types + same ambient occlusion
                    let block_hash = ao_index | (u32::from(current_voxel.id) << 9);
                    let data = data[axis]
                        .entry(block_hash)
                        .or_default()
                        .entry(y)
                        .or_default();
                    data[x] |= 1u32 << z as u32;
                }
            }
        }
    }

    data
}

#[must_use]
pub fn build_chunk_instance_data(chunks_refs: &ChunkRefs, lod: Lod) -> Option<ChunkMaterial> {
    // early exit, if all faces are culled
    if chunks_refs.is_all_voxels_same() {
        return None;
    }

    // solid binary for each x,y,z axis (3)
    #[allow(clippy::large_stack_arrays)]
    let mut axis_cols = [[[0u64; CHUNK_SIZE_P]; CHUNK_SIZE_P]; 3];

    // inner chunk voxels.
    let chunk = &*chunks_refs.adjacent_chunks[ChunkRefs::vec3_to_chunk_index(IVec3::new(1, 1, 1))];

    {
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;
        for i in 0..CHUNK_SIZE3 {
            add_voxel_to_axis_cols(
                chunk.get_block(i.into()),
                x + 1,
                y + 1,
                z + 1,
                &mut axis_cols,
            );

            x += 1;
            if x == CHUNK_SIZE {
                y += 1;
                x = 0;
                if y == CHUNK_SIZE {
                    z += 1;
                    y = 0;
                }
            }
        }
    }

    // neighbor chunk voxels.
    // note(leddoo): couldn't be bothered to optimize these.
    //  might be worth it though. together, they take
    //  almost as long as the entire "inner chunk" loop.
    for z in [0, CHUNK_SIZE_P - 1] {
        for y in 0..CHUNK_SIZE_P {
            for x in 0..CHUNK_SIZE_P {
                let pos = Position::new(x as i32 - 1, y as i32 - 1, z as i32 - 1);
                add_voxel_to_axis_cols(chunks_refs.get_block(pos), x, y, z, &mut axis_cols);
            }
        }
    }
    for z in 0..CHUNK_SIZE_P {
        for y in [0, CHUNK_SIZE_P - 1] {
            for x in 0..CHUNK_SIZE_P {
                let pos = Position::new(x as i32 - 1, y as i32 - 1, z as i32 - 1);
                add_voxel_to_axis_cols(chunks_refs.get_block(pos), x, y, z, &mut axis_cols);
            }
        }
    }
    for z in 0..CHUNK_SIZE_P {
        for x in [0, CHUNK_SIZE_P - 1] {
            for y in 0..CHUNK_SIZE_P {
                let pos = Position::new(x as i32 - 1, y as i32 - 1, z as i32 - 1);
                add_voxel_to_axis_cols(chunks_refs.get_block(pos), x, y, z, &mut axis_cols);
            }
        }
    }

    let data = calculate_ao(chunks_refs, &axis_cols);

    let mut quads: Vec<PackedQuad> = vec![];
    for (axis, block_ao_data) in data.into_iter().enumerate() {
        let face_dir = match axis {
            0 => FaceDir::Down,
            1 => FaceDir::Up,
            2 => FaceDir::Left,
            3 => FaceDir::Right,
            4 => FaceDir::Forward,
            _ => FaceDir::Back,
        };
        for (block_ao, axis_plane) in block_ao_data {
            let ao = block_ao & 0b111111111;
            for (axis_pos, plane) in axis_plane {
                for greedy_quad in greedy_mesh_binary_plane(plane, lod.size() as u32) {
                    let axis = axis_pos as i32;
                    let packed_quad = PackedQuad::new(
                        face_dir.world_to_sample(
                            axis,
                            greedy_quad.x as i32,
                            greedy_quad.y as i32,
                            lod,
                        ),
                        face_dir.normal_index(),
                        ao,
                        greedy_quad.h,
                        greedy_quad.w,
                    );
                    quads.push(packed_quad);
                }
            }
        }
    }

    if quads.is_empty() {
        return None;
    }

    Some(ChunkMaterial {
        quads,
        chunk_position: chunks_refs.center_chunk_position,
    })
}

#[derive(Debug)]
pub struct GreedyQuad {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// generate quads of a binary slice
/// lod not implemented atm
#[must_use]
pub fn greedy_mesh_binary_plane(mut data: [u32; CHUNK_SIZE], lod_size: u32) -> Vec<GreedyQuad> {
    let mut greedy_quads = vec![];
    for row in 0..data.len() {
        let mut y = 0;
        while y < lod_size {
            // find first solid, "air/zero's" could be first so skip
            y += (data[row] >> y).trailing_zeros();
            if y >= lod_size {
                // reached top
                continue;
            }
            let h = (data[row] >> y).trailing_ones();
            // convert height 'num' to positive bits repeated 'num' times aka:
            // 1 = 0b1, 2 = 0b11, 4 = 0b1111
            let h_as_mask = u32::checked_shl(1, h).map_or(!0, |v| v - 1);
            let mask = h_as_mask << y;
            // grow horizontally
            let mut w = 1;
            while row + w < lod_size as usize {
                // fetch bits spanning height, in the next row
                let next_row_h = (data[row + w] >> y) & h_as_mask;
                if next_row_h != h_as_mask {
                    break; // can no longer expand horizontally
                }

                // nuke the bits we expanded into
                data[row + w] &= !mask;

                w += 1;
            }
            greedy_quads.push(GreedyQuad {
                y,
                w: w as u32,
                h,
                x: row as u32,
            });
            y += h;
        }
    }
    greedy_quads
}
