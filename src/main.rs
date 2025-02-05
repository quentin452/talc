#![feature(stmt_expr_attributes)]

pub mod bevy;
pub mod chunky;
pub mod frustrum_culling;
pub mod mod_manager;
pub mod player;
pub mod position;
pub mod render;
pub mod smooth_transform;
pub mod sun;
pub mod utils;
pub mod winit;

use std::f32::consts::PI;

use bevy_app::TaskPoolThreadAssignmentPolicy;
use bevy_utils::default;
use render::RenderPlugin;
use winit::Winit;

use crate::bevy::prelude::*;

use crate::mod_manager::mod_loader::ModLoaderPlugin;
use crate::player::{
    debug_camera::NoCameraPlayerPlugin,
    render_distance::ScannerPlugin,
};
use crate::smooth_transform::smooth_transform;
use crate::{chunky::async_chunkloader::AsyncChunkloaderPlugin, sun::SunPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(RenderPlugin);
    app.add_plugins(TaskPoolPlugin {
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
    });
    app.add_plugins(AsyncChunkloaderPlugin);
    app.add_plugins(SunPlugin);
    app.add_plugins(ScannerPlugin);
    app.add_systems(Startup, setup);
    app.add_plugins(ModLoaderPlugin);
    app.add_plugins(NoCameraPlayerPlugin);
    app.add_systems(Update, smooth_transform);
    Winit::run(app);
}

pub fn setup(
    mut commands: Commands,
) {
    commands.spawn((
        Name::new("Sun"),
        crate::sun::Sun,
        /*DirectionalLight {
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            ..default()
        },*/
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 2., -PI / 4.)),
    ));
}
