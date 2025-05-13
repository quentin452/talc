use std::{sync::Arc, vec::Drain};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::primitives::Aabb,
    tasks::{block_on, AsyncComputeTaskPool, Task},
};

use crate::mod_manager::prototypes::BlockPrototypes;
use crate::position::{ChunkPosition, FloatingPosition};
use crate::{
    chunky::{
        chunk::{
            CHUNK_FLOAT_UP_BLOCKS_PER_SECOND, CHUNK_INITIAL_Y_OFFSET, CHUNK_SIZE_F32,
            CHUNK_SIZE_I32, ChunkData,
        },
        lod::Lod,
    },
    render::chunk_material::RenderableChunk,
};
use crate::{player::render_distance::Scanner, smooth_transform::SmoothTransformTo};
use futures_lite::future;

use super::{chunk::Chunk, chunks_refs::ChunkRefs, greedy_mesher_optimized};

pub struct AsyncChunkloaderPlugin;
impl Plugin for AsyncChunkloaderPlugin {
    fn build(&self, app: &mut App) {
        assert!(
            Lod::default().size() == CHUNK_SIZE_I32,
            "Default LOD must exactly equal the chunk size."
        );

        app.add_systems(Update, start_worldgen_threads);
        app.add_systems(Update, join_worldgen_threads);
        app.add_systems(Update, start_mesh_threads);
        app.add_systems(Update, join_mesh_threads);
        app.add_systems(Update, unload_chunks);
        app.add_systems(Update, unload_meshes);
        app.init_resource::<AsyncChunkloader>();
        app.init_resource::<Chunks>();
    }
}

pub const MAX_WORLDGEN_TASKS: usize = 64;
pub const MAX_MESH_TASKS: usize = 32;

#[derive(Resource, Default)]
pub struct Chunks(pub HashMap<ChunkPosition, Arc<ChunkData>>);

#[derive(Resource, Default)]
pub struct AsyncChunkloader {
    pub load_chunk_queue: Vec<ChunkPosition>,
    pub unload_chunk_queue: Vec<ChunkPosition>,
    pub load_mesh_queue: Vec<ChunkRefs>,
    pub unload_mesh_queue: Vec<ChunkPosition>,
    pub worldgen_tasks: HashMap<ChunkPosition, Task<ChunkData>>,
    pub mesh_tasks: HashMap<ChunkPosition, Task<Option<RenderableChunk>>>,
}

impl AsyncChunkloader {
    fn get_chunks_to_load(
        &mut self,
        player_position: FloatingPosition,
    ) -> Drain<'_, ChunkPosition> {
        let player_chunk_position: ChunkPosition = player_position.into();

        let tasks_left = (MAX_WORLDGEN_TASKS as i32 - self.worldgen_tasks.len() as i32)
            .min(self.load_chunk_queue.len() as i32)
            .max(0) as usize;

        self.load_chunk_queue.sort_by(|a, b| {
            a.0.distance_squared(player_chunk_position.0)
                .cmp(&b.0.distance_squared(player_chunk_position.0))
        });

        self.load_chunk_queue.drain(0..tasks_left)
    }

    fn get_chunks_to_unload(&mut self) -> Drain<'_, ChunkPosition> {
        self.unload_chunk_queue.drain(..)
    }

    fn get_chunks_to_mesh(&mut self, player_position: FloatingPosition) -> Drain<'_, ChunkRefs> {
        let player_chunk_position: ChunkPosition = player_position.into();

        let tasks_left = (MAX_MESH_TASKS as i32 - self.mesh_tasks.len() as i32)
            .min(self.load_mesh_queue.len() as i32)
            .max(0) as usize;

        self.load_mesh_queue.sort_by(|a, b| {
            a.center_chunk_position
                .0
                .distance_squared(player_chunk_position.0)
                .cmp(
                    &b.center_chunk_position
                        .0
                        .distance_squared(player_chunk_position.0),
                )
        });

        self.load_mesh_queue.drain(0..tasks_left)
    }

    fn get_chunks_to_unmesh(&mut self) -> Drain<'_, ChunkPosition> {
        self.unload_mesh_queue.drain(..)
    }
}

fn spawn_chunk_as_bevy_entity(
    chunk_data: ChunkData,
    chunk_entities: &mut Chunks,
    timer: &Time,
    commands: &mut Commands,
    chunk_canididates: Query<(Entity, &Chunk)>,
) {
    let chunk_position = chunk_data.position;
    for (entity_id, chunk) in chunk_canididates.iter() {
        if chunk.position == chunk_position {
            if let Ok(mut entity_commands) = commands.get_entity(entity_id) {
                entity_commands.despawn();
                break;
            }
        }
    }

    commands.spawn((
        Chunk {
            position: chunk_position,
        },
        SmoothTransformTo::new(
            timer,
            FloatingPosition::new(0., -CHUNK_INITIAL_Y_OFFSET, 0.),
            CHUNK_FLOAT_UP_BLOCKS_PER_SECOND,
        ),
        Aabb::from_min_max(Vec3::ZERO, Vec3::splat(CHUNK_SIZE_F32)),
        Transform::from_translation(
            (FloatingPosition::from(chunk_position)
                + FloatingPosition::new(0., CHUNK_INITIAL_Y_OFFSET, 0.))
            .0,
        ),
    ));

    chunk_entities
        .0
        .insert(chunk_position, Arc::new(chunk_data));
}

#[allow(clippy::needless_pass_by_value)]
fn start_worldgen_threads(
    mut chunkloader: ResMut<AsyncChunkloader>,
    block_prototypes: Res<BlockPrototypes>,
    scanners: Query<&GlobalTransform, With<Scanner>>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let scanner = scanners.single().unwrap();
    let player_position = FloatingPosition(scanner.translation());

    let to_load: Vec<ChunkPosition> = chunkloader.get_chunks_to_load(player_position).collect();
    for chunk_position in to_load {
        let prototypes = block_prototypes.clone();
        let task = task_pool.spawn(async move { ChunkData::generate(&prototypes, chunk_position) });
        chunkloader.worldgen_tasks.insert(chunk_position, task);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn join_worldgen_threads(
    mut chunkloader: ResMut<AsyncChunkloader>,
    mut chunk_entities: ResMut<Chunks>,
    timer: Res<Time>,
    mut commands: Commands,
    chunk_canididates: Query<(Entity, &Chunk)>,
) {
    chunkloader.worldgen_tasks.retain(|_, task| {
        // check on our worldgen task to see how it's doing :)
        let status = block_on(future::poll_once(task));

        // keep the entry in our task vector only if the task is not done yet
        let retain = status.is_none();

        // if this task is done, handle the data it returned!
        if let Some(chunk_component) = status {
            spawn_chunk_as_bevy_entity(chunk_component, &mut chunk_entities, &timer, &mut commands, chunk_canididates);
        }

        retain
    });
}

#[allow(clippy::needless_pass_by_value)]
fn start_mesh_threads(
    mut chunkloader: ResMut<AsyncChunkloader>,
    scanners: Query<&GlobalTransform, With<Scanner>>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let scanner = scanners.single().unwrap();
    let player_position = FloatingPosition(scanner.translation());

    let to_mesh: Vec<ChunkRefs> = chunkloader.get_chunks_to_mesh(player_position).collect();
    for chunk_refs in to_mesh {
        let k = chunk_refs.center_chunk_position;
        let task = task_pool.spawn(async move {
            greedy_mesher_optimized::build_chunk_instance_data(
                &chunk_refs,
                super::lod::Lod::default(),
            )
        });
        chunkloader.mesh_tasks.insert(k, task);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn join_mesh_threads(
    mut chunkloader: ResMut<AsyncChunkloader>,
    chunk_canididates: Query<(Entity, &Chunk)>,
    mut commands: Commands,
) {
    chunkloader.mesh_tasks.retain(|chunk_position, task| {
        // check on our mesh task to see how it's doing :)
        let status = block_on(future::poll_once(task));

        // keep the entry in our task vector only if the task is not done yet
        let Some(renderable_chunk_optional) = status else {
            return true;
        };

        // if this task is done, handle the data it returned!
        if let Some(renderable_chunk) = renderable_chunk_optional {
            // todo: refactor to use bevy indexes when the update drops.
            for (entity_id, chunk) in chunk_canididates.iter() {
                if chunk.position == *chunk_position {
                    if let Ok(mut entity_commands) = commands.get_entity(entity_id) {
                        entity_commands.insert(renderable_chunk);
                        break;
                    }
                }
            }
        }

        false
    });
}

#[allow(clippy::needless_pass_by_value)]
fn unload_chunks(
    mut chunkloader: ResMut<AsyncChunkloader>,
    mut chunk_entities: ResMut<Chunks>,
    chunk_canididates: Query<(Entity, &Chunk)>,
    mut commands: Commands,
) {
    let to_unload: HashSet<ChunkPosition> = chunkloader.get_chunks_to_unload().collect();

    // todo: refactor to use bevy indexes when the update drops.
    for (entity_id, chunk) in chunk_canididates.iter() {
        if to_unload.contains(&chunk.position) {
            if let Ok(mut entity_commands) = commands.get_entity(entity_id) {
                entity_commands.despawn();
            }
        }
    }

    for chunk_position in to_unload {
        chunk_entities.0.remove(&chunk_position);
        chunkloader.worldgen_tasks.remove(&chunk_position);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn unload_meshes(
    mut chunkloader: ResMut<AsyncChunkloader>,
    mut commands: Commands,
    chunk_canididates: Query<(Entity, &Chunk)>,
) {
    let to_unload: HashSet<ChunkPosition> = chunkloader.get_chunks_to_unmesh().collect();

    // todo: refactor to use bevy indexes when the update drops.
    for (entity_id, chunk) in chunk_canididates.iter() {
        if to_unload.contains(&chunk.position) {
            if let Ok(mut entity_commands) = commands.get_entity(entity_id) {
                entity_commands.try_remove::<RenderableChunk>();
            }
        }
    }
}
