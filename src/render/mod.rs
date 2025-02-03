use bevy_window::{PrimaryWindow, Window};
use chunk_render_pipeline::ChunkRenderPipeline;
use wgpu_context::{draw, RenderDevice, WgpuContext};

use crate::bevy::prelude::*;

pub mod chunk_material;
pub mod chunk_render_pipeline;
pub mod wgpu_context;
pub mod depth_texture;

// When writing custom rendering code it's generally recommended to use a plugin.
// The main reason for this is that it gives you access to the finish() hook
// which is called after rendering resources are initialized.
pub struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        let mut world = app.world_mut();
        let window = world.query_filtered::<&Window, With<PrimaryWindow>>().get_single(world);
        let window = window.expect("Failed to find primary window.");
        let wgpu_context = WgpuContext::new(world, window);
        let render_device = world.resource::<RenderDevice>();
        app.insert_resource(ChunkRenderPipeline::new(render_device, &wgpu_context.surface_config));
        app.add_systems(Update, draw);
    }
}