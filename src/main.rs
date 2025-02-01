use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::{
    app::TaskPoolThreadAssignmentPolicy,
    core_pipeline::bloom::Bloom,
    pbr::{Atmosphere, AtmosphereSettings},
    render::{
        RenderPlugin,
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    },
};

use talc::player::{
    debug_camera::{FlyCam, NoCameraPlayerPlugin},
    render_distance::Scanner,
    render_distance::ScannerPlugin,
};
use talc::{
    chunky::chunk::{CHUNK_SIZE_I32, CHUNK_SIZE2},
    mod_manager::{
        mod_loader::ModLoaderPlugin,
        prototypes::{BlockPrototypes, Prototypes},
    },
    position::FloatingPosition,
    rendering::{
        ChunkMaterial, ChunkMaterialWireframe, GlobalChunkMaterial, GlobalChunkWireframeMaterial,
        RenderingPlugin,
    },
};
use talc::{
    position::RelativePosition,
    sun::SunPlugin,
    chunky::async_chunkloader::{AsyncChunkloader, AsyncChunkloaderPlugin},
};
use talc::smooth_transform::smooth_transform;

use rand::Rng;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            })
            .set(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    async_compute: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: 8,
                        percent: 0.75,
                        on_thread_spawn: None,
                        on_thread_destroy: None,
                    },
                    ..default()
                },
            }),))
        .add_plugins(AsyncChunkloaderPlugin)
        .add_plugins(SunPlugin)
        .add_plugins(ScannerPlugin)
        .add_systems(Startup, setup)
        .add_plugins(RenderingPlugin)
        .add_plugins(ModLoaderPlugin)
        .add_plugins(NoCameraPlayerPlugin)
        .add_systems(Update, modify_current_terrain)
        .add_systems(Update, smooth_transform)
        .run();
}

#[allow(clippy::needless_pass_by_value)]
pub fn modify_current_terrain(
    query: Query<&Transform, With<Camera>>,
    key: Res<ButtonInput<KeyCode>>,
    mut voxel_engine: ResMut<AsyncChunkloader>,
    block_prototypes: Res<BlockPrototypes>,
) {
    /*if !key.pressed(KeyCode::KeyN) {
        return;
    }
    let camera_transform = query.single();
    let looking_at_position = camera_transform.translation + (camera_transform.forward() * 64.0);
    let looking_at_position = FloatingPosition(looking_at_position);
    let camera_chunk = looking_at_position.into();

    let mut rng = rand::rng();
    let mut mods = vec![];
    for _ in 0..CHUNK_SIZE2 {
        let pos = RelativePosition::new(
            rng.random_range(0..CHUNK_SIZE_I32),
            rng.random_range(0..CHUNK_SIZE_I32),
            rng.random_range(0..CHUNK_SIZE_I32),
        );
        mods.push(ChunkModification(pos, block_prototypes.get("air").unwrap()));
    }
    voxel_engine.chunk_modifications.insert(camera_chunk, mods);*/
}

pub fn setup(
    mut commands: Commands,
    mut chunk_materials: ResMut<Assets<ChunkMaterial>>,
    mut chunk_materials_wireframe: ResMut<Assets<ChunkMaterialWireframe>>,
    #[allow(unused)] mut materials: ResMut<Assets<StandardMaterial>>,
    #[allow(unused)] mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((
        Name::new("Sun"),
        talc::sun::Sun,
        DirectionalLight {
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 2., -PI / 4.)),
    ));

    commands
        .spawn((
            Scanner::new(12),
            Transform::from_xyz(0.0, 200.0, 0.5),
            Camera3d::default(),
            FlyCam,
            Camera {
                hdr: true,
                ..default()
            },
            Atmosphere {
                bottom_radius: 5_000.0,
                top_radius: 64_600.0 * 3.,
                ground_albedo: Vec3::splat(0.3),
                rayleigh_density_exp_scale: 1.0 / 8_000.0,
                rayleigh_scattering: Vec3::new(5.802e-5, 13.558e-5, 33.100e-5),
                mie_density_exp_scale: 1.0 / 1_200.0,
                mie_scattering: 3.996e-6,
                mie_absorption: 0.444e-6,
                mie_asymmetry: 0.8,
                ozone_layer_altitude: 25_000.0,
                ozone_layer_width: 30_000.0,
                ozone_absorption: Vec3::new(0.650e-6, 1.881e-6, 0.085e-6),
            },
            AtmosphereSettings {
                aerial_view_lut_max_distance: 3.2,
                scene_units_to_m: 1.,
                ..Default::default()
            },
            //Tonemapping::AgX,
            Bloom::NATURAL,
        ))
        .insert(FlyCam);

    commands.insert_resource(GlobalChunkMaterial(chunk_materials.add(ChunkMaterial {
        reflectance: 0.5,
        perceptual_roughness: 1.0,
        metallic: 0.01,
    })));

    commands.insert_resource(GlobalChunkWireframeMaterial(chunk_materials_wireframe.add(
        ChunkMaterialWireframe {
            reflectance: 0.5,
            perceptual_roughness: 1.0,
            metallic: 0.01,
        },
    )));
}
