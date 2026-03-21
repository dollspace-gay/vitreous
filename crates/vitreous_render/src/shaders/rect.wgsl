// Rounded rectangle SDF shader — fill + stroke + per-corner border radius.
//
// Uses signed distance to the rounded rectangle boundary:
//   distance < 0 → inside
//   distance > 0 → outside
// Anti-aliasing via smoothstep over a 1-pixel band at the boundary.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct RectInstance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,   // tl, tr, br, bl
    @location(4) border_color: vec4<f32>,
    @location(5) border_width: f32,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    @location(5) border_width: f32,
}

// Quad vertices (two triangles) indexed by vertex_index 0..5
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
    instance: RectInstance,
) -> VertexOutput {
    let uv = quad_vertex(vertex_index);

    // Expand quad by 1px in all directions for anti-aliasing
    let padding = vec2<f32>(1.0, 1.0);
    let world_pos = instance.pos - padding + uv * (instance.size + padding * 2.0);

    // Convert from pixel coordinates to clip space: [0, viewport] -> [-1, 1]
    let ndc = vec2<f32>(
        world_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - world_pos.y / globals.viewport_size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.local_pos = uv * instance.size + (uv - 0.5) * padding * 2.0;
    out.size = instance.size;
    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color;
    out.border_width = instance.border_width;
    return out;
}

// Signed distance to a rounded rectangle centered at the origin.
// `half_size` is half the rectangle dimensions.
// `radii` = (top_left, top_right, bottom_right, bottom_left)
fn sdf_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select the radius for the quadrant that `p` is in.
    var r: f32;
    if p.x < 0.0 {
        if p.y < 0.0 {
            r = radii.x; // top-left
        } else {
            r = radii.w; // bottom-left
        }
    } else {
        if p.y < 0.0 {
            r = radii.y; // top-right
        } else {
            r = radii.z; // bottom-right
        }
    }

    // Clamp radius to half the smallest dimension
    r = min(r, min(half_size.x, half_size.y));

    let q = abs(p) - half_size + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_size = in.size * 0.5;
    let center = half_size;
    let p = in.local_pos - center;

    let dist = sdf_rounded_rect(p, half_size, in.border_radius);

    // Anti-aliased edge: smoothstep over ~1 pixel
    let aa = 1.0 - smoothstep(-0.5, 0.5, dist);

    if in.border_width > 0.0 {
        // Border rendering: fill between outer and inner SDF
        let inner_half = half_size - vec2<f32>(in.border_width, in.border_width);
        let inner_radii = max(in.border_radius - vec4<f32>(in.border_width), vec4<f32>(0.0));
        let inner_dist = sdf_rounded_rect(p, inner_half, inner_radii);
        let inner_aa = 1.0 - smoothstep(-0.5, 0.5, inner_dist);

        // Border region = inside outer but outside inner
        let border_mask = aa * (1.0 - inner_aa);
        let fill_mask = aa * inner_aa;

        let color = in.color * fill_mask + in.border_color * border_mask;
        return color;
    } else {
        return vec4<f32>(in.color.rgb, in.color.a * aa);
    }
}
