use std::sync::Arc;
use bevy_window::{PrimaryWindow, Window};
use wgpu::MemoryHints::Performance;
use crate::{bevy::prelude::*, player::camera::Camera};
use super::{chunk_material::BakedChunkMesh, chunk_render_pipeline::ChunkRenderPipeline, depth_texture::{depth_texture, Material}};

#[derive(Resource, Deref, Clone)]
pub struct RenderDevice(pub Arc<wgpu::Device>);

#[derive(Resource)]
pub struct WgpuContext {
    pub device: Arc<wgpu::Device>,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub queue: wgpu::Queue,
    pub object_pipeline: Option<wgpu::RenderPipeline>,
    pub depth_texture: Material,
}

impl<'window> WgpuContext {
    pub async fn new_async(window: &'static winit::window::Window) -> WgpuContext {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .expect("Unable to create WGPU surface.");
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::BUFFER_BINDING_ARRAY | wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY,
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    #[allow(clippy::unnecessary_struct_initialization)]
                    required_limits: wgpu::Limits{
                        ..wgpu::Limits::default()
                    }.using_resolution(adapter.limits()),
                    memory_hints: Performance,
                },
                None,
            )
            .await
            .expect("Failed to create GPU device connection.");

        let device = Arc::new(device);

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let surface_config = surface
            .get_default_config(&adapter, width, height)
            .expect("Requested adapter will support created surface.");
        surface.configure(&device, &surface_config);

        let depth_texture = depth_texture(&device, &surface_config);

        WgpuContext {
            surface,
            surface_config,
            queue,
            object_pipeline: None,
            depth_texture,
            device
        }
    }

    pub fn new(window: &'static winit::window::Window) -> Self {
        block_on(WgpuContext::new_async(window))
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        let (width, height) = new_size;
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_texture = depth_texture(&self.device, &self.surface_config);
    }
}

pub fn draw(
    cameras: Query<&Camera>,
    render_device: Res<RenderDevice>,
    chunk_render_pipeline: Res<ChunkRenderPipeline>,
    wgpu_context: Res<WgpuContext>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    to_draw: Query<&BakedChunkMesh>
) {
    if let Ok(window) = primary_window.get_single() {
        let aspect_ratio = (f64::from(window.width()) / f64::from(window.height())) as f32;

        for camera in cameras.iter() {
            let surface_texture = wgpu_context
                .surface
                .get_current_texture()
                .expect("Failed to acquire next swap chain texture");
            let texture_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.6,
                            g: 0.9,
                            b: 1.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &wgpu_context.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&chunk_render_pipeline);

            let baked_camera = camera.bake(&render_device, aspect_ratio);
            render_pass.set_bind_group(0, &baked_camera.bind_group, &[]);

            for chunk in to_draw.iter() {
                chunk.render(&mut render_pass);
            }
            
            std::mem::drop(render_pass);

            wgpu_context.queue.submit(Some(encoder.finish()));
            surface_texture.present();
        }
    }
}