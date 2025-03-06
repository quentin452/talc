use bevy::{
    core_pipeline::core_3d::{CORE_3D_DEPTH_FORMAT, Transparent3d},
    ecs::system::{
        SystemParamItem,
        lifetimeless::{Read, SRes},
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey, RenderMeshInstances,
        SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        mesh::{
            MeshVertexAttribute, MeshVertexBufferLayoutRef, RenderMesh, allocator::MeshAllocator,
        },
        render_asset::RenderAssets,
        render_phase::{
            DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
            SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            BindGroupLayout, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState,
            Face, FragmentState, FrontFace, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, RenderPipelineDescriptor, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines, TextureFormat, VertexFormat,
            VertexState,
        },
        renderer::RenderDevice,
        view::{ExtractedView, RenderVisibleEntities, ViewTarget},
    },
};

use super::chunk_material::{BakedChunkMesh, bind_group_layout};

const SHADER_ASSET_PATH: &str = "shaders/chunk.wgsl";

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
pub(super) fn queue_custom_mesh_pipeline(
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
    for (view_visible_entities, view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        // Create the key based on the view. In this case we only care about MSAA
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for &(render_entity, visible_entity) in view_visible_entities.get::<BakedChunkMesh>().iter()
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
                self.bind_group_layout.clone(),
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

pub(super) struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<BakedChunkMesh>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        baked_chunk_mesh: Option<&'w BakedChunkMesh>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(baked_chunk_mesh) = baked_chunk_mesh else {
            return RenderCommandResult::Skip;
        };

        baked_chunk_mesh.render(pass);
        panic!("aa");
        RenderCommandResult::Success
    }
}
