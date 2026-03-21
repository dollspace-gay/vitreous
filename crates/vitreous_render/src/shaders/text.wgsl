// Text rendering shader — textured quads from glyph atlas.
//
// Each glyph is a quad with UV coordinates into the glyph atlas texture.
// The atlas stores alpha-only (coverage) values; the text color is applied
// in the fragment shader.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var glyph_texture: texture_2d<f32>;
@group(1) @binding(1) var glyph_sampler: sampler;

struct GlyphInstance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_min: vec2<f32>,
    @location(3) uv_max: vec2<f32>,
    @location(4) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

fn quad_vertex(index: u32) -> vec2<f32> {
    switch index {
        case 0u: { return vec2<f32>(0.0, 0.0); }
        case 1u: { return vec2<f32>(1.0, 0.0); }
        case 2u: { return vec2<f32>(0.0, 1.0); }
        case 3u: { return vec2<f32>(1.0, 0.0); }
        case 4u: { return vec2<f32>(1.0, 1.0); }
        default: { return vec2<f32>(0.0, 1.0); }
    }
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: GlyphInstance,
) -> VertexOutput {
    let local = quad_vertex(vertex_index);
    let world_pos = instance.pos + local * instance.size;

    let ndc = vec2<f32>(
        world_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - world_pos.y / globals.viewport_size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = mix(instance.uv_min, instance.uv_max, local);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(glyph_texture, glyph_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * coverage);
}
