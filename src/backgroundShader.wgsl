struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    var out: VertexOutput;

    var uv = vec2f(f32((id << 1) & 2), f32(id & 2));
    out.clip_position = vec4f(uv * vec2f(2, 2) - vec2f(1, 1), 0, 1);
    out.tex_coords = vec2f(uv.x, 1.0 - uv.y);

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}