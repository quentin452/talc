/*!
scanner is responsible for identifying what chunks needs to be loaded (mesh/data)
the current implementation is exellent for low render distances, 1-15
but anything above that might induce some frame lag, due to how the load/unload data is calculated.
`scanner::new()` can also be very slow on high render distances, giving an initial slow execution time.
*/

use std::collections::VecDeque;

use bevy::{prelude::*, platform_support::collections::HashSet};

use crate::{
    chunk::CHUNK_SIZE_I32, constants::ADJACENT_CHUNK_DIRECTIONS, position::ChunkPosition,
    utils::index_to_ivec3_bounds, voxel_engine::VoxelEngine,
};

pub const MAX_DATA_TASKS: usize = 9;
pub const MAX_MESH_TASKS: usize = 3;

pub const MAX_SCANS: usize = 26000;

pub struct ScannerPlugin;

impl Plugin for ScannerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                detect_move,
                scan_data,
                scan_data_unload,
                scan_mesh_unload,
                scan_mesh,
            ),
        );
    }
}

#[derive(Component)]
pub struct Scanner {
    pub prev_chunk_pos: ChunkPosition,
    /// how many chunks we visit
    pub checks_per_frame: usize,
    /// offset grid sampling over frames
    pub data_offset: usize,
    /// offset grid sampling over frames
    pub mesh_offset: usize,

    // chunk positions we are yet to check we need need to load
    pub unresolved_data_load: Vec<ChunkPosition>,
    pub unresolved_mesh_load: Vec<ChunkPosition>,

    // chunk positions we are yet to check we need need tounload
    pub unresolved_data_unload: VecDeque<ChunkPosition>,
    pub unresolved_mesh_unload: VecDeque<ChunkPosition>,

    // on detecting a scanner move, these offsets are used to
    // identify the location of what chunks need to be checked
    pub data_sampling_offsets: Vec<ChunkPosition>,
    pub mesh_sampling_offsets: Vec<ChunkPosition>,
}

impl Scanner {
    /// construct scanner, chunk offsets are based on distance
    /// warning: slow execution time on distances above 15-20,
    #[must_use]
    pub fn new(distance: i32) -> Self {
        let data_distance = distance + 1;
        let mesh_distance = distance;
        let data_sampling_offsets = make_offset_vec(data_distance);
        let mesh_sampling_offsets = make_offset_vec(mesh_distance);
        Self {
            checks_per_frame: 32 * 32 * 32,
            data_offset: 0,
            data_sampling_offsets,
            mesh_sampling_offsets,
            mesh_offset: 0,
            unresolved_data_load: Vec::default(),
            prev_chunk_pos: ChunkPosition::new(777, 777, 777),
            unresolved_mesh_load: Vec::default(),
            unresolved_data_unload: VecDeque::default(),
            unresolved_mesh_unload: VecDeque::default(),
        }
    }
}

/// on scanner chunk change, enqueue chunks to load/unload
fn detect_move(
    mut scanners: Query<(&mut Scanner, &GlobalTransform)>,
    mut voxel_engine: ResMut<VoxelEngine>,
) {
    for (mut scanner, g_transform) in &mut scanners {
        let chunk_pos = (g_transform.translation().as_ivec3() - IVec3::splat(CHUNK_SIZE_I32 / 2))
            / CHUNK_SIZE_I32;
        let chunk_pos = ChunkPosition(chunk_pos);
        let previous_chunk_pos = scanner.prev_chunk_pos;
        let chunk_pos_changed = chunk_pos != scanner.prev_chunk_pos;
        scanner.prev_chunk_pos = chunk_pos;
        if !chunk_pos_changed {
            return;
        }
        let load_data_area = scanner
            .data_sampling_offsets
            .iter()
            .map(|offset| chunk_pos + *offset)
            .collect::<HashSet<ChunkPosition>>();

        let unload_data_area = scanner
            .data_sampling_offsets
            .iter()
            .map(|offset| previous_chunk_pos + *offset)
            .collect::<HashSet<ChunkPosition>>();

        let load_mesh_area = scanner
            .mesh_sampling_offsets
            .iter()
            .map(|offset| chunk_pos + *offset)
            .collect::<HashSet<ChunkPosition>>();

        let unload_mesh_area = scanner
            .mesh_sampling_offsets
            .iter()
            .map(|offset| previous_chunk_pos + *offset)
            .collect::<HashSet<ChunkPosition>>();

        let data_load = load_data_area.difference(&unload_data_area);
        let data_unload = unload_data_area.difference(&load_data_area);
        let mesh_load = load_mesh_area.difference(&unload_mesh_area);
        let mesh_unload = unload_mesh_area.difference(&load_mesh_area);

        scanner.unresolved_data_load.extend(data_load);
        scanner.unresolved_data_unload.extend(data_unload);
        scanner.unresolved_mesh_unload.extend(mesh_unload);
        scanner.unresolved_mesh_load.extend(mesh_load);

        // deconstruct scanner mutable references because rust :P
        let Scanner {
            unresolved_data_load,
            unresolved_mesh_load,
            unresolved_data_unload,
            unresolved_mesh_unload,
            ..
        } = scanner.as_mut();

        for p in unresolved_mesh_unload.iter() {
            if let Some((i, _)) = voxel_engine
                .load_mesh_queue
                .iter()
                .enumerate()
                .find(|(_i, k)| *k == p)
            {
                voxel_engine.load_mesh_queue.remove(i);
            }
        }
        for p in unresolved_data_unload.iter() {
            if let Some((i, _)) = voxel_engine
                .load_data_queue
                .iter()
                .enumerate()
                .find(|(_i, k)| *k == p)
            {
                voxel_engine.load_data_queue.remove(i);
            }
        }

        // remove the unloads from load
        unresolved_mesh_load.retain(|p| {
            let want_unload = unresolved_mesh_unload.contains(p);
            !want_unload
        });
        // remove the unloads from load
        unresolved_data_load.retain(|p| {
            let want_unload = unresolved_data_unload.contains(p);
            !want_unload
        });

        scanner.unresolved_mesh_load.sort_by(|a, b| {
            a.0.distance_squared(chunk_pos.0)
                .cmp(&b.0.distance_squared(chunk_pos.0))
        });
        scanner.unresolved_data_load.sort_by(|a, b| {
            a.0.distance_squared(chunk_pos.0)
                .cmp(&b.0.distance_squared(chunk_pos.0))
        });
    }
}

/// constructs spherical chunk positions with the provided chunk radius
fn make_offset_vec(half: i32) -> Vec<ChunkPosition> {
    let k = (half * 2) + 1;
    let mut sampling_offsets = vec![];
    for i in 0..k * k * k {
        let mut pos = index_to_ivec3_bounds(i, k);
        pos -= IVec3::splat((k as f32 * 0.5) as i32);
        sampling_offsets.push(ChunkPosition(pos));
    }
    sampling_offsets.sort_by(|a, b| {
        a.0.distance_squared(IVec3::ZERO)
            .cmp(&b.0.distance_squared(IVec3::ZERO))
    });
    sampling_offsets
}

pub fn scan_data(
    mut scanners: Query<(&mut Scanner, &GlobalTransform)>,
    mut voxel_engine: ResMut<VoxelEngine>,
) {
    for (mut scanner, _g_transform) in &mut scanners {
        if voxel_engine.data_tasks.len() >= MAX_DATA_TASKS {
            return;
        }
        let l = scanner.unresolved_data_load.len();
        // for chunk_pos in scanner.unresolved_data_load.drain(..) {
        for chunk_pos in scanner.unresolved_data_load.drain(0..MAX_SCANS.min(l)) {
            // want to load chunk
            let is_busy = voxel_engine.world_data.contains_key(&chunk_pos)
                || voxel_engine.load_data_queue.contains(&chunk_pos)
                || voxel_engine.data_tasks.contains_key(&chunk_pos);
            if !is_busy {
                voxel_engine.load_data_queue.push(chunk_pos);
                // abort unload
                let index_of_unloading = voxel_engine
                    .unload_data_queue
                    .iter()
                    .enumerate()
                    .find_map(|(i, pos)| if pos == &chunk_pos { Some(i) } else { None });
                if let Some(i) = index_of_unloading {
                    voxel_engine.unload_data_queue.remove(i);
                }
            }
        }
    }
}

pub fn scan_data_unload(
    mut scanners: Query<(&mut Scanner, &GlobalTransform)>,
    mut voxel_engine: ResMut<VoxelEngine>,
) {
    // find all loaded and check if in range
    for (mut scanner, _g_transform) in &mut scanners {
        for chunk_pos in scanner.unresolved_data_unload.drain(..) {
            // want to load chunk
            let is_busy = !voxel_engine.world_data.contains_key(&chunk_pos);
            if !is_busy {
                voxel_engine.unload_data_queue.push(chunk_pos);
            }
        }
    }
}

pub fn scan_mesh_unload(mut scanners: Query<&mut Scanner>, mut voxel_engine: ResMut<VoxelEngine>) {
    // find all loaded and check if in range
    for mut scanner in &mut scanners {
        for chunk_pos in scanner.unresolved_mesh_unload.drain(..) {
            voxel_engine.unload_mesh_queue.push(chunk_pos);
        }
    }
}

pub fn scan_mesh(mut scanners: Query<&mut Scanner>, mut voxel_engine: ResMut<VoxelEngine>) {
    for mut scanner in &mut scanners {
        // if voxel_engine.data_tasks.len() >= MAX_MESH_TASKS {
        //     return;
        // }
        let mut retries = Vec::new();
        let l = scanner.unresolved_mesh_load.len();
        for chunk_pos in scanner.unresolved_mesh_load.drain(0..MAX_SCANS.min(l)) {
            let mut busy = voxel_engine.load_mesh_queue.contains(&chunk_pos);
            // all data available
            busy |= !ADJACENT_CHUNK_DIRECTIONS
                .iter()
                .map(|of| chunk_pos + *of)
                .all(|p| voxel_engine.world_data.contains_key(&p));

            if busy {
                retries.push(chunk_pos);
            } else {
                voxel_engine.load_mesh_queue.push(chunk_pos);
                // abort unload
                let index_of_unloading = voxel_engine
                    .unload_mesh_queue
                    .iter()
                    .enumerate()
                    .find_map(|(i, pos)| if pos == &chunk_pos { Some(i) } else { None });
                if let Some(i) = index_of_unloading {
                    voxel_engine.unload_mesh_queue.remove(i);
                }
            }
        }
        scanner.unresolved_mesh_load.append(&mut retries);
    }
}
