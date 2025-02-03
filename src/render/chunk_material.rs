//! A shader that renders a mesh multiple times in one draw call.
//!
//! Bevy will automatically batch and instance your meshes assuming you use the same
//! `Handle<Material>` and `Handle<Mesh>` for all of your instances.
//!
//! This example is intended for advanced users and shows how to make a custom instancing
//! implementation using bevy's low level rendering api.
//! It's generally recommended to try the built-in instancing before going with this approach.

use crate::{bevy::prelude::*, position::ChunkPosition};
use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::position::RelativePosition;

use super::wgpu_context::RenderDevice;

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/chunk.wgsl";

/// In talc we draw quads instead of triangles.
/// This struct repersents bit packed data for each quad ready to be sent to the GPU.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct PackedQuad {
    /// Repersents bit-packed instance data for every quad.
    /// FORMAT
    /// x: 00000 (5)
    /// y: 00000 (10)
    /// z: 00000 (15)
    /// normal: 000 (18)
    /// ao: 00 (20)
    /// x strech: 00000 (25)
    /// y strech: 00000 (30)
    /// 2 bits are free :)
    packed_u32: u32,
}

impl PackedQuad {
    #[inline]
    #[must_use]
    pub fn new(
        position: RelativePosition,
        normal: u32,
        _ao: u32,
        x_strech: u32,
        y_strech: u32,
    ) -> PackedQuad {
        let x = position.x();
        let y = position.y();
        let z = position.z();

        let ao = 0; // todo
        let x_strech = x_strech.min(31);
        let y_strech = y_strech.min(31);

        #[rustfmt::skip]
        {
            debug_assert!(0 <= position.x() && position.x() < 32, "x position out of range. expected 0..=31, got {x}");
            debug_assert!(0 <= position.y() && position.y() < 32, "y position out of range. expected 0..=31, got {y}");
            debug_assert!(0 <= position.z() && position.z() < 32, "z position out of range. expected 0..=31, got {z}");
            debug_assert!(normal < 6, "normal out of range. expected 0..=6, got {normal}");
            debug_assert!(ao < 4, "ao out of range. expected 0..=3, got {ao}");
            debug_assert!(x_strech < 32, "x strech out of range. expected 0..=31, got {x_strech}");
            debug_assert!(y_strech < 32, "y strech out of range. expected 0..=31, got {y_strech}");
        }

        let packed_u32: u32 = x as u32
            | ((y as u32) << 5u32)
            | ((z as u32) << 10u32)
            | (normal << 15u32)
            | (ao << 18u32)
            | (x_strech << 20u32)
            | (y_strech << 25u32);

        Self { packed_u32 }
    }
}

/// Bufferized and GPU-ready version of a chunk.
/// Each chunk in the render world ECS holds one of these.
#[derive(Component)]
pub struct BakedChunkMesh {
    instance_buffer: Buffer,
    instance_buffer_length: usize,
    uniform_bind_group: BindGroup,
    simple_quad_index_buffer: SimpleQuad,
}

impl BakedChunkMesh {
    pub fn render(&self, render_pass: &mut RenderPass) {
        render_pass.set_index_buffer(
            self.simple_quad_index_buffer.index_buffer.slice(..),
            IndexFormat::Uint32,
        );
        render_pass.set_vertex_buffer(
            0,
            self.simple_quad_index_buffer.vertex_buffer.slice(..),
        );
        render_pass.set_vertex_buffer(
            1,
            self.instance_buffer.slice(..)
        );
        render_pass.draw_indexed(
            0..self.simple_quad_index_buffer.length,
            0,
            0..self.instance_buffer_length as u32,
        );
    }
}

/// Is trivally converted to `BakedChunkMesh`.
pub struct ChunkMaterial {
    pub quads: Vec<PackedQuad>,
    pub chunk_position: ChunkPosition,
}

impl ChunkMaterial {
    pub fn bake(&self, render_device: &RenderDevice) -> BakedChunkMesh {
        let instance_buffer = render_device.create_buffer_init(&BufferInitDescriptor {
            label: Some("chunk per-instance data buffer"),
            contents: bytemuck::cast_slice(self.quads.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let uniform_buffer = render_device.create_buffer_init(&BufferInitDescriptor {
            label: Some("chunk uniform buffer"),
            contents: bytemuck::cast_slice(&self.chunk_position.0.to_array()),
            usage: BufferUsages::UNIFORM,
        });
        let uniform_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("chunk bind group"),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
            layout: &bind_group_layout(render_device),
        });
        
        BakedChunkMesh {
            instance_buffer,
            uniform_bind_group,
            instance_buffer_length: self.quads.len(),
            simple_quad_index_buffer: SimpleQuad::new(render_device),
        }
    }
}

pub fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("chunk uniform buffer bind ground layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

#[derive(Resource)]
struct SimpleQuad {
    index_buffer: Buffer,
    vertex_buffer: Buffer,
    length: u32,
}

impl SimpleQuad {
    fn new(render_device: &RenderDevice) -> Self {
        const SQUARE_VERTICES: &[[f32; 3]] = &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ];
        let vertex_buffer = render_device.create_buffer_init(&BufferInitDescriptor {
            label: Some("generic quad index buffer"),
            contents: bytemuck::cast_slice(SQUARE_VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = render_device.create_buffer_init(&BufferInitDescriptor {
            label: Some("generic quad index buffer"),
            contents: bytemuck::cast_slice(&[0, 1, 2, 3, 2, 1]),
            usage: BufferUsages::INDEX,
        });
        Self {
            index_buffer,
            vertex_buffer,
            length: 6,
        }
    }
}