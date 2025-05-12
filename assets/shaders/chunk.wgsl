#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#import bevy_pbr::{
    forward_io::{FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip, mesh_normal_local_to_world}
#import bevy_pbr::pbr_functions::{calculate_view, prepare_world_normal}
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::pbr_types::pbr_input_new
#import bevy_pbr::view_transformations::position_world_to_clip

@group(1) @binding(0)
var<uniform> chunk_position: vec3<i32>;

struct InstanceInput {
    @location(0) constant_quad: vec3<f32>,
};

struct VertexInput {
    @location(1) vert_data: u32,
    //@builtin(vertex_index) vertex_index: u32
};

var<private> ambient_lerps: vec4<f32> = vec4<f32>(1.0,0.7,0.5,0.15);

// indexing an array has to be in some memory
// by declaring this as a var instead it works
var<private> normals: array<vec3<f32>, 6> = array<vec3<f32>, 6> (
	vec3<f32>(-1.0, 0.0, 0.0), // Left
	vec3<f32>(1.0, 0.0, 0.0), // Right
	vec3<f32>(0.0, -1.0, 0.0), // Down
	vec3<f32>(0.0, 1.0, 0.0), // Up
	vec3<f32>(0.0, 0.0, -1.0), // Forward
	vec3<f32>(0.0, 0.0, 1.0) // Back
);

fn x_positive_bits(bits: u32) -> u32 {
    return (1u << bits) - 1u;
}

@vertex
fn vertex(vertex: VertexInput, instance_input: InstanceInput) -> VertexOutput {
    let x_strech = (vertex.vert_data >> 20u & x_positive_bits(5u)) + 1;
    let y_strech = (vertex.vert_data >> 25u & x_positive_bits(5u)) + 1;
    var x = f32(vertex.vert_data & x_positive_bits(5u)) + f32(chunk_position.x * 32);
    var y = f32(vertex.vert_data >> 5u & x_positive_bits(5u)) + f32(chunk_position.y * 32);
    var z = f32(vertex.vert_data >> 10u & x_positive_bits(5u)) + f32(chunk_position.z * 32);
    let normal_index = vertex.vert_data >> 15u & x_positive_bits(3u);

    switch normal_index {
        case 0u: { // left
            y += instance_input.constant_quad.x * f32(x_strech) - 1;
            x += 0.0;
            z += instance_input.constant_quad.z * f32(y_strech);
        }
        case 1u: { // right
            y += instance_input.constant_quad.z * f32(x_strech) - 1;
            x += 1.0;
            z += instance_input.constant_quad.x * f32(y_strech);
        }
        case 2u: { // down
            x += instance_input.constant_quad.z * f32(y_strech);
            y += -1.0;
            z += instance_input.constant_quad.x * f32(x_strech);
        }
        case 3u, default: { // up
            x += instance_input.constant_quad.x * f32(y_strech);
            y += 0.0;
            z += instance_input.constant_quad.z * f32(x_strech);
        }
        case 4u { // forward
            x += instance_input.constant_quad.x * f32(y_strech);
            z += 0.0;
            y += instance_input.constant_quad.z * f32(x_strech) - 1;
        }
        case 5u { // backward
            x += instance_input.constant_quad.z * f32(y_strech);
            z += 1.0;
            y += instance_input.constant_quad.x * f32(x_strech) - 1;
        }
    }
    let ao = vertex.vert_data >> 18u & x_positive_bits(2u);

    var out: VertexOutput;
    out.normal = normals[normal_index];
    out.ambient = ao;
    //out.position = vec3<f32>(x,y,z);
    out.clip_position = position_world_to_clip(vec3<f32>(x,y,z));

    return out;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) blend_color: vec3<f32>,
    @location(3) ambient: u32,
};

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = vec4<f32>(in.normal, 1.0);
    
    let light = Light(
        vec3<f32>(0.0, 100.0, 0.0),
        vec3<f32>(1.0, 1.0, 1.0),
    );

    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    let light_dir = normalize(light.position - in.position);

    let diffuse_strength = max(dot(in.normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let result = (ambient_color + diffuse_color) * object_color.xyz;
    return vec4<f32>(result, object_color.a);
}