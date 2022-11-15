
// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;
@group(0) @binding(1)
var<uniform> time: f32;

@group(1) @binding(0)
var<uniform> audio_in: f32;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) index: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) index: f32,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = camera.view_proj * vec4<f32>(model.position.xyz, 1.0); // 2.
    out.index = model.index;
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(sin(time + in.index + 3.0)/2.0 + 0.5, cos(1.5*time + in.index)/2.0 + 0.5, sin(sin(time + in.index + 1.0)*3.14)/2.0 + 0.5, 0.0);
}