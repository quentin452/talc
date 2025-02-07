struct Camera {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> chunk_position: vec3<i32>;

struct VertexInput {
    @location(0) constant_quad: vec3<f32>,
    @location(1) vert_data: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) blend_color: vec3<f32>,
    @location(3) ambient: f32,
};

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

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
fn vertex(vertex: VertexInput) -> VertexOutput {
    let x = i32(vertex.vert_data & x_positive_bits(5u)) + (chunk_position.x * 32) + i32(vertex.constant_quad.x);
    let y = i32(vertex.vert_data >> 5u & x_positive_bits(5u)) + (chunk_position.y * 32) + i32(vertex.constant_quad.y);
    let z = i32(vertex.vert_data >> 10u & x_positive_bits(5u)) + (chunk_position.z * 32) + i32(vertex.constant_quad.z);
    //let ao = vertex.vert_data >> 18u & x_positive_bits(3u);
    let ao = 0.0;
    //let normal_index = vertex.vert_data >> 21u & x_positive_bits(3u);
    let normal_index = 0;
    //let block_index = vertex.vert_data >> 25u & x_positive_bits(7u);

    var out: VertexOutput;
    out.normal = normals[normal_index];
    out.clip_position = camera.view_proj * vec4<f32>(f32(x), f32(y), f32(z), 1.0);
    out.position = vec3<f32>(f32(x), f32(y), f32(z));
    out.ambient = ao;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);

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