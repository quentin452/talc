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

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) vert_data: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec4<f32>,
    @location(2) blend_color: vec3<f32>,
    @location(3) ambient: f32,
    @location(4) instance_index: u32,
};

var<private> ambient_lerps: vec4<f32> = vec4<f32>(1.0,0.7,0.5,0.15);

// indexing an array has to be in some memory
// by declaring this as a var instead it works
var<private> normals: array<vec3<f32>,6> = array<vec3<f32>,6> (
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

@group(2) @binding(0)
var<uniform> chunk_position: vec3<i32>;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let x = f32(vertex.vert_data & x_positive_bits(6u));
    let y = f32(vertex.vert_data >> 6u & x_positive_bits(6u));
    let z = f32(vertex.vert_data >> 12u & x_positive_bits(6u));
    let ao = vertex.vert_data >> 18u & x_positive_bits(3u);
    let normal_index = vertex.vert_data >> 21u & x_positive_bits(3u);
    //let block_index = vertex.vert_data >> 25u & x_positive_bits(7u);

    let local_position = vec4<f32>(x,y,z, 1.0);
    let world_position = vec4<f32>(f32(chunk_position.x * 32), f32(chunk_position.y * 32), f32(chunk_position.z * 32), 1.0);
    out.clip_position = position_world_to_clip(world_position.xyz + local_position.xyz);

    let ambient_lerp = ambient_lerps[ao];
    out.ambient = ambient_lerp;
    out.world_position = world_position;

    let normal = normals[normal_index];
    out.world_normal = mesh_normal_local_to_world(normal, vertex.instance_index);

    let s = 0.05;
    var noise = simplexNoise2(vec2<f32>(world_position.x*s, world_position.z*s));
    var k = simplexNoise2(vec2<f32>(world_position.x*s, world_position.z*s));

    let high = vec3<f32>(9.00, 6.0, 0.0);
    let low = vec3<f32>(0.8, 1.0, 0.40);
    noise = (out.world_position.y) / 30.0;
    
    let fun = (low * noise) + (high * (1.0-noise));
    out.blend_color = vec3<f32>(0.3, 0.4, 0.0);
    out.instance_index = vertex.instance_index;
    return out;
}

@fragment
fn fragment(input: VertexOutput) -> FragmentOutput {
    var pbr_input = pbr_input_new();

    pbr_input.flags = mesh[input.instance_index].flags;

    pbr_input.V = calculate_view(input.world_position, false);
    pbr_input.frag_coord = input.clip_position;
    pbr_input.world_position = input.world_position;

    pbr_input.world_normal = prepare_world_normal(
        input.world_normal,
        false,
        false,
    );

    pbr_input.N = normalize(pbr_input.world_normal);

    pbr_input.material.base_color = vec4<f32>(input.blend_color * input.ambient, 1.0);

    pbr_input.material.reflectance = vec3<f32>(0.5, 0.5, 0.5);
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.metallic = 0.1;

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    return out;
}

//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket, Johan Helsing
//
fn mod289(x: vec2f) -> vec2f {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn mod289_3(x: vec3f) -> vec3f {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn permute3(x: vec3f) -> vec3f {
    return mod289_3(((x * 34.) + 1.) * x);
}

//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
fn simplexNoise2(v: vec2f) -> f32 {
    let C = vec4(
        0.211324865405187, // (3.0-sqrt(3.0))/6.0
        0.366025403784439, // 0.5*(sqrt(3.0)-1.0)
        -0.577350269189626, // -1.0 + 2.0 * C.x
        0.024390243902439 // 1.0 / 41.0
    );

    // First corner
    var i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);

    // Other corners
    var i1 = select(vec2(0., 1.), vec2(1., 0.), x0.x > x0.y);

    // x0 = x0 - 0.0 + 0.0 * C.xx ;
    // x1 = x0 - i1 + 1.0 * C.xx ;
    // x2 = x0 - 1.0 + 2.0 * C.xx ;
    var x12 = x0.xyxy + C.xxzz;
    x12.x = x12.x - i1.x;
    x12.y = x12.y - i1.y;

    // Permutations
    i = mod289(i); // Avoid truncation effects in permutation

    var p = permute3(permute3(i.y + vec3(0., i1.y, 1.)) + i.x + vec3(0., i1.x, 1.));
    var m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3(0.));
    m *= m;
    m *= m;

    // Gradients: 41 points uniformly over a line, mapped onto a diamond.
    // The ring size 17*17 = 289 is close to a multiple of 41 (41*7 = 287)
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;

    // Normalize gradients implicitly by scaling m
    // Approximation of: m *= inversesqrt( a0*a0 + h*h );
    m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);

    // Compute final noise value at P
    let g = vec3(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}