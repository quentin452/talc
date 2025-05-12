use bevy::{
    core_pipeline::core_3d::{Transparent3d, CORE_3D_DEPTH_FORMAT},
    ecs::system::{
        lifetimeless::{Read, SRes}, SystemParamItem
    },
    pbr::{MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey, SetMeshViewBindGroup},
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, mesh::{PrimitiveTopology, VertexBufferLayout}, render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        }, render_resource::{
            BindGroupLayout, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState,
            Face, FragmentState, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, RenderPipelineDescriptor, SpecializedRenderPipeline,
            SpecializedRenderPipelines, TextureFormat, VertexAttribute, VertexFormat, VertexState,
            VertexStepMode,
        }, renderer::RenderDevice, sync_world::MainEntity, view::{ExtractedView, RenderVisibleEntities, ViewTarget}, Render, RenderApp, RenderSet
    },
};

use super::chunk_material::{RenderableChunk, bind_group_layout};

const SHADER_ASSET_PATH: &str = "shaders/chunk.wgsl";

// When writing custom rendering code it's generally recommended to use a plugin.
// The main reason for this is that it gives you access to the finish() hook
// which is called after rendering resources are initialized.
pub struct ChunkRenderPipelinePlugin;
impl Plugin for ChunkRenderPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<RenderableChunk>::default()); // TODO

        // We make sure to add these to the render app, not the main app.
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_render_command::<Transparent3d, DrawCustom>();
        render_app.init_resource::<SpecializedRenderPipelines<CustomPipeline>>();
        render_app.add_systems(
            Render,
            (
                queue_custom_render_pipeline.in_set(RenderSet::Queue),
                //prepare_instance_buffers.in_set(RenderSet::PrepareResources),
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

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
fn queue_custom_render_pipeline(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&RenderVisibleEntities, &ExtractedView, &Msaa)>,
    material_meshes: Query<(Entity, &MainEntity, &RenderableChunk)>,
) {
    // Get the id for our custom draw function
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (_, view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        // Create the key based on the view. In this case we only care about MSAA
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for (render_entity, visible_entity, renderable_chunk) in &material_meshes // TODO: frustrum culling. see https://github.com/bevyengine/bevy/blob/19ee692f9621f89f305096f423507e925b748b9a/examples/shader/specialized_mesh_pipeline.rs#L353
        {
            // Specialize the key for the current mesh entity
            // For this example we only specialize based on the mesh topology
            // but you could have more complex keys and that's where you'd need to create those keys
            let key = view_key
                | MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

            // Finally, we can specialize the pipeline based on the key
            let pipeline = pipelines.specialize(&pipeline_cache, &custom_pipeline, key);

            // Add the mesh with our specialized pipeline
            transparent_phase.add(Transparent3d {
                entity: (render_entity, *visible_entity),
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder.distance_translation(&renderable_chunk.chunk_position().map(|x| x * 32).as_vec3()),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Resource)]
pub(super) struct CustomPipeline {
    shader_handle: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = bind_group_layout(render_device);
        let mesh_pipeline = world.resource::<MeshPipeline>();

        CustomPipeline {
            shader_handle: world.load_asset(SHADER_ASSET_PATH),
            mesh_pipeline: mesh_pipeline.clone(),
            bind_group_layout: bind_group_layout,
        }
    }
}

/// The custom draw commands that Bevy executes for each entity we enqueue into
/// the render phase.
pub(super) type DrawCustom = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform at bind group 0
    SetMeshViewBindGroup<0>,
    DrawChunk,
);

// Set a custom vertex buffer layout for our render pipeline.
impl SpecializedRenderPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Define a buffer layout for our vertex buffer. Our vertex buffer only has one entry which is a packed u32
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }
            ],
        };

        let instance_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<u32>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 0,
                    shader_location: 1,
                }
            ],
        };
        
        RenderPipelineDescriptor {
            label: Some("Specialized Mesh Pipeline".into()),
            layout: vec![
                // Bind group 0 is the view uniform
                self.mesh_pipeline
                    .get_view_layout(MeshPipelineViewLayoutKey::from(key))
                    .clone(),
                // Bind group 1 is the chunk position.
                self.bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                // Customize how to store the meshes' vertex attributes in the vertex buffer
                buffers: vec![vertex_buffer_layout, instance_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    // This isn't required, but bevy supports HDR and non-HDR rendering
                    // so it's generally recommended to specialize the pipeline for that
                    format: if key.contains(MeshPipelineKey::HDR) {
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
                topology: PrimitiveTopology::TriangleList,
                front_face: bevy::render::render_resource::FrontFace::Ccw,
                cull_mode: Some(Face::Front),
                unclipped_depth: false,
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
                count: key.msaa_samples(),
                ..MultisampleState::default()
            },
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub(super) struct DrawChunk;

impl<P: PhaseItem> RenderCommand<P> for DrawChunk {
    type Param = (SRes<RenderDevice>,);
    type ViewQuery = ();
    type ItemQuery = Read<RenderableChunk>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        renderable_chunk: Option<&'w RenderableChunk>,
        (ref render_device,): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(renderable_chunk) = renderable_chunk else {
            return RenderCommandResult::Skip;
        };
        renderable_chunk.render(render_device, pass);
        RenderCommandResult::Success
    }
}
