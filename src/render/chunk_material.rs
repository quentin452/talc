//! A shader that renders a mesh multiple times in one draw call.
//!
//! Bevy will automatically batch and instance your meshes assuming you use the same
//! `Handle<Material>` and `Handle<Mesh>` for all of your instances.
//!
//! This example is intended for advanced users and shows how to make a custom instancing
//! implementation using bevy's low level rendering api.
//! It's generally recommended to try the built-in instancing before going with this approach.

use std::sync::Arc;

use bevy::{
    core_pipeline::core_3d::{CORE_3D_DEPTH_FORMAT, Transparent3d},
    ecs::system::{SystemParamItem, lifetimeless::*},
    pbr::{
        MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey, RenderMeshInstances,
        SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{
            MeshVertexAttribute, MeshVertexBufferLayoutRef, RenderMesh,
            allocator::MeshAllocator,
        },
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::{self, ExtractedView, RenderVisibleEntities, ViewTarget, VisibilityClass},
    },
};
use bytemuck::{Pod, Zeroable};

use crate::position::{ChunkPosition, Position};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/chunk.wgsl";

// When writing custom rendering code it's generally recommended to use a plugin.
// The main reason for this is that it gives you access to the finish() hook
// which is called after rendering resources are initialized.
pub struct CustomChunkMaterialPlugin;
impl Plugin for CustomChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<RenderableChunk>::default());

        // We make sure to add these to the render app, not the main app.
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_render_command::<Transparent3d, DrawCustom>();
        render_app.init_resource::<SpecializedMeshPipelines<CustomPipeline>>();
        render_app.add_systems(
            Render,
            (
                queue_custom_mesh_pipeline.in_set(RenderSet::QueueMeshes),
                prepare_instance_buffers.in_set(RenderSet::PrepareResources),
            ),
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        // Creating this pipeline needs the RenderDevice and RenderQueue
        // which are only available once rendering plugins are initialized.
        render_app.init_resource::<CustomPipeline>();
    }
}

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
        position: Position,
        normal: u32,
        ao: u32,
        x_strech: u32,
        y_strech: u32,
    ) -> PackedQuad {
        let x = position.x;
        let y = position.y;
        let z = position.z;

        let ao = 0; // todo
        let x_strech = x_strech.min(31);
        let y_strech = y_strech.min(31);

        #[rustfmt::skip]
        {
            debug_assert!(0 <= position.x && position.x < 32, "x position out of range. expected 0..=31, got {x}");
            debug_assert!(0 <= position.y && position.y < 32, "y position out of range. expected 0..=31, got {y}");
            debug_assert!(0 <= position.z && position.z < 32, "z position out of range. expected 0..=31, got {z}");
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

/// Note the [`ExtractComponent`] trait implementation: this is necessary to
/// tell Bevy that this object should be pulled into the render world. Also note
/// the `on_add` hook, which is needed to tell Bevy's `check_visibility` system
/// that entities with this component need to be examined for visibility.
#[derive(Clone, Component, ExtractComponent, Deref)]
#[require(VisibilityClass)]
#[component(on_add = view::add_visibility_class::<RenderableChunk>)]
pub struct RenderableChunk(pub Arc<ChunkMaterial>);

/// This struct does not exist in render world.
/// It is trivally converted to `ChunkInstanceBuffer` when passed to render world via the `prepare_instance_buffers` system.
pub struct ChunkMaterial {
    pub quads: Vec<PackedQuad>,
    pub chunk_position: ChunkPosition,
}

/// Bufferized and GPU-ready version of a chunk.
/// Each chunk in the render world ECS holds one of these.
#[derive(Component)]
struct ChunkInstanceBuffer {
    buffer: Buffer,
    uniform_bind_group: BindGroup,
    length: usize,
    simple_quad_index_buffer: SimpleQuadIndexBuffer,
}

impl ChunkInstanceBuffer {
    pub fn new(
        render_device: &RenderDevice,
        chunk_material: &ChunkMaterial,
        bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("chunk per-instance data buffer"),
            contents: bytemuck::cast_slice(chunk_material.quads.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("chunk uniform buffer"),
            contents: bytemuck::cast_slice(&chunk_material.chunk_position.0.to_array()),
            usage: BufferUsages::UNIFORM,
        });
        let uniform_bind_group =
            render_device.create_bind_group("chunk bind group", &bind_group_layout, &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ]);
        let index_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("generic quad index buffer"),
            contents: bytemuck::cast_slice(&[0, 1, 2, 3, 2, 1]),
            usage: BufferUsages::INDEX,
        });
        let simple_quad_index_buffer = SimpleQuadIndexBuffer {
            buffer: index_buffer,
            length: 6,
        };
        ChunkInstanceBuffer {
            buffer: instance_buffer,
            uniform_bind_group,
            length: chunk_material.quads.len(),
            simple_quad_index_buffer,
        }
    }
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &RenderableChunk)>,
    render_device: Res<RenderDevice>,
    bind_group_layout: Res<ChunkUniformBufferBindGroupLayout>,
) {
    for (render_entity, renderable_chunk) in &query {
        let instance_buffer =
            ChunkInstanceBuffer::new(&render_device, &renderable_chunk.0, &bind_group_layout);
        commands.entity(render_entity).insert(instance_buffer);
    }
}

#[derive(Resource, Deref, Clone)]
struct ChunkUniformBufferBindGroupLayout(BindGroupLayout);

impl FromWorld for ChunkUniformBufferBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout =
            render_device.create_bind_group_layout("chunk uniform buffer bind ground layout", &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]);
        ChunkUniformBufferBindGroupLayout(bind_group_layout)
    }
}

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
fn queue_custom_mesh_pipeline(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&RenderVisibleEntities, &ExtractedView, &Msaa)>,
) {
    // Get the id for our custom draw function
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (view_visible_entities, view, msaa) in views.iter() {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        // Create the key based on the view. In this case we only care about MSAA
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for &(render_entity, visible_entity) in
            view_visible_entities.get::<ChunkInstanceBuffer>().iter()
        {
            // Get the mesh instance
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(visible_entity)
            else {
                continue;
            };

            // Get the mesh data
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            // Specialize the key for the current mesh entity
            // For this example we only specialize based on the mesh topology
            // but you could have more complex keys and that's where you'd need to create those keys
            let key =
                view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            // Finally, we can specialize the pipeline based on the key
            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                // This should never with this example, but if your pipeline specialization
                // can fail you need to handle the error here
                .expect("Failed to specialize mesh pipeline");

            // Add the mesh with our specialized pipeline
            transparent_phase.add(Transparent3d {
                entity: (render_entity, visible_entity),
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder.distance_translation(&mesh_instance.translation),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Resource)]
struct SimpleQuadIndexBuffer {
    buffer: Buffer,
    length: u32,
}

#[derive(Resource)]
struct CustomPipeline {
    shader_handle: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: ChunkUniformBufferBindGroupLayout,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        world.init_resource::<ChunkUniformBufferBindGroupLayout>();
        let bind_group_layout = world.resource::<ChunkUniformBufferBindGroupLayout>();
        let mesh_pipeline = world.resource::<MeshPipeline>();

        CustomPipeline {
            shader_handle: world.load_asset(SHADER_ASSET_PATH),
            mesh_pipeline: mesh_pipeline.clone(),
            bind_group_layout: bind_group_layout.clone(),
        }
    }
}

/// The custom draw commands that Bevy executes for each entity we enqueue into
/// the render phase.
type DrawCustom = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform at bind group 0
    SetMeshViewBindGroup<0>,
    // Set the mesh uniform at bind group 1
    SetMeshBindGroup<1>,
    // Draw the mesh
    DrawMeshInstanced,
);

// A "high" random id should be used for custom attributes to ensure consistent sorting and avoid collisions with other attributes.
// See the MeshVertexAttribute docs for more info.
pub const ATTRIBUTE_VOXEL: MeshVertexAttribute =
    MeshVertexAttribute::new("Voxel", 988540919, VertexFormat::Uint32);

// Set a custom vertex buffer layout for our render pipeline.
impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        mesh_key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        // Define a buffer layout for our vertex buffer. Our vertex buffer only has one entry which is a packed u32
        let vertex_buffer_layout = layout
            .0
            .get_layout(&[ATTRIBUTE_VOXEL.at_shader_location(0)])?;

        Ok(RenderPipelineDescriptor {
            label: Some("Specialized Mesh Pipeline".into()),
            layout: vec![
                // Bind group 0 is the view uniform
                self.mesh_pipeline
                    .get_view_layout(MeshPipelineViewLayoutKey::from(mesh_key))
                    .clone(),
                // Bind group 1 is the mesh uniform
                self.mesh_pipeline.mesh_layouts.model_only.clone(),
                // Bind group 2 is our custom chunk uniform.
                self.bind_group_layout.0.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                // Customize how to store the meshes' vertex attributes in the vertex buffer
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    // This isn't required, but bevy supports HDR and non-HDR rendering
                    // so it's generally recommended to specialize the pipeline for that
                    format: if mesh_key.contains(MeshPipelineKey::HDR) {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    // For this example we only use opaque meshes,
                    // but if you wanted to use alpha blending you would need to set it here
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: mesh_key.primitive_topology(),
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                conservative: false, // Enabling this requires `Features::CONSERVATIVE_RASTERIZATION` to be enabled.
                ..default()
            },
            // Note that if your view has no depth buffer this will need to be
            // changed.
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: default(),
                bias: default(),
            }),
            // It's generally recommended to specialize your pipeline for MSAA,
            // but it's not always possible
            multisample: MultisampleState {
                count: mesh_key.msaa_samples(),
                ..MultisampleState::default()
            },
            zero_initialize_workgroup_memory: false,
        })
    }
}

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = &'static ChunkInstanceBuffer;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w ChunkInstanceBuffer>,
        (): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };

        pass.set_bind_group(2, &instance_buffer.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        //pass.set_vertex_buffer(1, instance_buffer.instance_buffer.slice(..));
        
        pass.set_index_buffer(
            instance_buffer.simple_quad_index_buffer.buffer.slice(..),
            0,
            IndexFormat::Uint32
        );
        pass.draw_indexed(
            0..instance_buffer.simple_quad_index_buffer.length,
            vertex_buffer_slice.range.start as i32,
            0..instance_buffer.length as u32,
        );
        RenderCommandResult::Success
    }
}
