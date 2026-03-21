// Box shadow shader — blurred SDF for soft shadow rendering.
//
// Renders a shadow by evaluating the rounded rectangle SDF and applying
// a gaussian-like blur approximation. The shadow quad is expanded beyond
// the element bounds by (blur_radius + spread_radius) to accommodate
// the full shadow extent.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct ShadowInstance {
    @location(0) pos: vec2<f32>,         // top-left of shadow quad (includes offset + expand)
    @location(1) size: vec2<f32>,        // size of shadow quad
    @location(2) rect_pos: vec2<f32>,    // top-left of the source rectangle
    @location(3) rect_size: vec2<f32>,   // size of the source rectangle (after spread)
    @location(4) color: vec4<f32>,
    @location(5) border_radius: vec4<f32>,  // tl, tr, br, bl
    @location(6) blur_radius: f32,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
    @location(1) rect_pos: vec2<f32>,
    @location(2) rect_size: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) border_radius: vec4<f32>,
    @location(5) blur_radius: f32,
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
    instance: ShadowInstance,
) -> VertexOutput {
    let local = quad_vertex(vertex_index);
    let world_pos = instance.pos + local * instance.size;

    let ndc = vec2<f32>(
        world_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - world_pos.y / globals.viewport_size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.world_pos = world_pos;
    out.rect_pos = instance.rect_pos;
    out.rect_size = instance.rect_size;
    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.blur_radius = instance.blur_radius;
    return out;
}

fn sdf_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    var r: f32;
    if p.x < 0.0 {
        if p.y < 0.0 {
            r = radii.x;
        } else {
            r = radii.w;
        }
    } else {
        if p.y < 0.0 {
            r = radii.y;
        } else {
            r = radii.z;
        }
    }
    r = min(r, min(half_size.x, half_size.y));

    let q = abs(p) - half_size + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_size = in.rect_size * 0.5;
    let center = in.rect_pos + half_size;
    let p = in.world_pos - center;

    let dist = sdf_rounded_rect(p, half_size, in.border_radius);

    // Approximate gaussian blur using smoothstep over the blur radius.
    // The transition band is proportional to blur_radius for soft edges.
    let sigma = max(in.blur_radius * 0.5, 0.5);
    let alpha = 1.0 - smoothstep(-sigma, sigma, dist);

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
