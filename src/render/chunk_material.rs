//! A shader that renders a mesh multiple times in one draw call.
//!
//! Bevy will automatically batch and instance your meshes assuming you use the same
//! `Handle<Material>` and `Handle<Mesh>` for all of your instances.
//!
//! This example is intended for advanced users and shows how to make a custom instancing
//! implementation using bevy's low level rendering api.
//! It's generally recommended to try the built-in instancing before going with this approach.

use bevy::{
    core_pipeline::core_3d::{Transparent3d, CORE_3D_DEPTH_FORMAT},
    ecs::system::{lifetimeless::*, SystemParamItem},
    pbr::{
        MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup
    },
    prelude::*,
    render::{
        view::{self, ExtractedView, RenderVisibleEntities, ViewTarget, VisibilityClass},
        extract_component::{ExtractComponent, ExtractComponentPlugin}, mesh::{
            allocator::MeshAllocator, MeshVertexAttribute, MeshVertexBufferLayoutRef, RenderMesh, RenderMeshBufferInfo
        }, render_asset::RenderAssets, render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        }, render_resource::*, renderer::RenderDevice, Render, RenderApp, RenderSet
    },
};
use bytemuck::{Pod, Zeroable};

use crate::position::ChunkPosition;

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/chunk.wgsl";


/// A marker component that represents an entity that is to be rendered using
/// our specialized pipeline.
///
/// Note the [`ExtractComponent`] trait implementation: this is necessary to
/// tell Bevy that this object should be pulled into the render world. Also note
/// the `on_add` hook, which is needed to tell Bevy's `check_visibility` system
/// that entities with this component need to be examined for visibility.
#[derive(Clone, Component, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = view::add_visibility_class::<ChunkMaterial>)]
pub struct ChunkMaterial {
    pub instance_data: Vec<ChunkInstanceData>,
    pub chunk_position: ChunkPosition
}

#[derive(Bundle)]
pub struct RenderableChunk {
    pub chunk_material: ChunkMaterial,
    pub mesh: Mesh3d
}

// When writing custom rendering code it's generally recommended to use a plugin.
// The main reason for this is that it gives you access to the finish() hook
// which is called after rendering resources are initialized.
pub struct CustomChunkMaterialPlugin;
impl Plugin for CustomChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<ChunkMaterial>::default());

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

/// This struct contains constant data for each chunk. It is accessible for every vertex in the chunk.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ChunkInstanceData {
    pub chunk_position: IVec3,
}

/// Bufferized and GPU-ready version of the above^ struct.
#[derive(Component)]
struct ChunkInstanceBuffer {
    instance_buffer: Buffer,
    uniform_bind_group: BindGroup,
    length: usize
}

fn uniform_buffer_bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        "chunk uniform buffer bind ground layout",
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }]
    )
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &ChunkMaterial)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, chunk_instance_data) in &query {
        let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("chunk per-instance data buffer"),
            contents: bytemuck::cast_slice(chunk_instance_data.instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("chunk uniform buffer"),
            contents: bytemuck::cast_slice(&chunk_instance_data.chunk_position.0.to_array()),
            usage: BufferUsages::UNIFORM,
        });
        let bind_group_entry = BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: &uniform_buffer,
                offset: 0,
                size: None
            })
        };
        let uniform_bind_group = render_device.create_bind_group(
            "chunk bind group",
            &uniform_buffer_bind_group_layout(&render_device),
            &[bind_group_entry]
        );
        commands.entity(entity).insert(ChunkInstanceBuffer {
            instance_buffer,
            uniform_bind_group,
            length: chunk_instance_data.instance_data.len(),
        });
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
        for &(render_entity, visible_entity) in view_visible_entities.get::<ChunkMaterial>().iter() {
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
struct CustomPipeline {
    shader_handle: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    render_device: RenderDevice
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let mesh_pipeline = world.resource::<MeshPipeline>();

        CustomPipeline {
            shader_handle: world.load_asset(SHADER_ASSET_PATH),
            mesh_pipeline: mesh_pipeline.clone(),
            render_device: world.resource::<RenderDevice>().clone()
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
                uniform_buffer_bind_group_layout(&self.render_device)
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
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // A borrow check workaround.
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_bind_group(2, &instance_buffer.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        //pass.set_vertex_buffer(1, instance_buffer.instance_buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    //0..instance_buffer.length as u32,
                    0..1u32,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
