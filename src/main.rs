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

use talc::debug_menu::FpsCounterPlugin;
use talc::mod_manager::mod_loader::ModLoaderPlugin;
use talc::player::{
    debug_camera::{FlyCam, NoCameraPlayerPlugin},
    render_distance::Scanner,
    render_distance::ScannerPlugin,
};
use talc::render::chunk_render_pipeline::ChunkRenderPipelinePlugin;
use talc::smooth_transform::smooth_transform;
use talc::{chunky::async_chunkloader::AsyncChunkloaderPlugin, sun::SunPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            })
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
        .add_plugins(ModLoaderPlugin)
        .add_plugins(NoCameraPlayerPlugin)
        .add_systems(Update, smooth_transform)
        .add_plugins(ChunkRenderPipelinePlugin)
        .add_plugins(FpsCounterPlugin)
        .run();
}

pub fn setup(
    mut commands: Commands,
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
}
