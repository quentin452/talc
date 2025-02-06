use bevy_utils::default;
use wgpu::*;

use crate::{bevy::prelude::*, player::camera::Camera};

use super::wgpu_context::RenderDevice;

#[derive(Resource, Deref)]
pub(super) struct ChunkRenderPipeline(RenderPipeline);

impl ChunkRenderPipeline {
    pub fn new(render_device: &RenderDevice, surface_config: &SurfaceConfiguration) -> Self {
        // Define a buffer layout for our vertex buffer. This buffer stores a constant quad.
        let constant_quad_vertex_buffer_layout = VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        };

        // instanced data. each instance only needs a single U32. see `chunk_material::PackedQuad` for binary format.
        let instanced_vertex_buffer_layout = VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        };                

        let shader: wgpu::ShaderModule =
            render_device.create_shader_module(include_wgsl!("../../assets/shaders/chunk.wgsl"));

        let bind_group_layouts = [
            &Camera::bind_group_layout(render_device),
            &crate::render::chunk_material::bind_group_layout(render_device),
        ];

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Chunk Render Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let fragment_targets = [Some(surface_config.format.into())];

        let descriptor = RenderPipelineDescriptor {
            label: Some("Specialized Chunk Mesh Pipeline".into()),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                buffers: &[constant_quad_vertex_buffer_layout, instanced_vertex_buffer_layout],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &fragment_targets,
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                conservative: false, // Enabling this requires `Features::CONSERVATIVE_RASTERIZATION` to be enabled.
                ..default()
            },
            // Note that if your view has no depth buffer this will need to be
            // changed.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        };

        let pipeline = render_device.create_render_pipeline(&descriptor);
        Self(pipeline)
    }
}
