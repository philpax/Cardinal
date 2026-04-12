// NanoVG wgpu shader — ported from GL2 fillVertShader / fillFragShader.
//
// Bind group 0: per-frame uniforms
//   binding 0 — ViewUniforms  (vertex + fragment)
//   binding 1 — FragUniforms  (fragment)
// Bind group 1: texture
//   binding 0 — texture_2d<f32>
//   binding 1 — sampler

// ── Uniform structs ─────────────────────────────────────────────────

struct ViewUniforms {
    view_size: vec2<f32>,
};

// Matches GL2 frag[11] layout — 11 vec4s.
struct FragUniforms {
    frag: array<vec4<f32>, 11>,
};

@group(0) @binding(0) var<uniform> view: ViewUniforms;
@group(0) @binding(1) var<uniform> fu: FragUniforms;

@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var tex_sampler: sampler;

// ── Vertex ──────────────────────────────────────────────────────────

struct VertexInput {
    @location(0) vertex: vec2<f32>,
    @location(1) tcoord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ftcoord: vec2<f32>,
    @location(1) fpos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.ftcoord = in.tcoord;
    out.fpos = in.vertex;
    out.position = vec4<f32>(
        2.0 * in.vertex.x / view.view_size.x - 1.0,
        1.0 - 2.0 * in.vertex.y / view.view_size.y,
        0.0,
        1.0,
    );
    return out;
}

// ── Fragment helpers ────────────────────────────────────────────────

// Reconstruct mat3 from three vec4 rows (using .xyz of each).
fn scissor_mat() -> mat3x3<f32> {
    return mat3x3<f32>(
        fu.frag[0].xyz,
        fu.frag[1].xyz,
        fu.frag[2].xyz,
    );
}

fn paint_mat() -> mat3x3<f32> {
    return mat3x3<f32>(
        fu.frag[3].xyz,
        fu.frag[4].xyz,
        fu.frag[5].xyz,
    );
}

fn inner_col() -> vec4<f32> { return fu.frag[6]; }
fn outer_col() -> vec4<f32> { return fu.frag[7]; }
fn scissor_ext() -> vec2<f32> { return fu.frag[8].xy; }
fn scissor_scale() -> vec2<f32> { return fu.frag[8].zw; }
fn extent() -> vec2<f32> { return fu.frag[9].xy; }
fn radius() -> f32 { return fu.frag[9].z; }
fn feather() -> f32 { return fu.frag[9].w; }
fn stroke_mult() -> f32 { return fu.frag[10].x; }
fn stroke_thr() -> f32 { return fu.frag[10].y; }
fn tex_type() -> i32 { return i32(fu.frag[10].z); }
fn frag_type() -> i32 { return i32(fu.frag[10].w); }

fn sdroundrect(pt: vec2<f32>, ext: vec2<f32>, rad: f32) -> f32 {
    let ext2 = ext - vec2<f32>(rad, rad);
    let d = abs(pt) - ext2;
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0))) - rad;
}

fn scissor_mask(p: vec2<f32>) -> f32 {
    let sc_raw = (scissor_mat() * vec3<f32>(p, 1.0)).xy;
    let sc = vec2<f32>(0.5, 0.5) - (abs(sc_raw) - scissor_ext()) * scissor_scale();
    return clamp(sc.x, 0.0, 1.0) * clamp(sc.y, 0.0, 1.0);
}

fn stroke_mask(ftcoord: vec2<f32>) -> f32 {
    return min(1.0, (1.0 - abs(ftcoord.x * 2.0 - 1.0)) * stroke_mult()) * min(1.0, ftcoord.y);
}

// Core shading logic shared by both AA and non-AA fragment entry points.
fn shade(ftcoord: vec2<f32>, fpos: vec2<f32>, stroke_alpha: f32) -> vec4<f32> {
    let scissor = scissor_mask(fpos);
    let ft = frag_type();

    if ft == 0 {
        // Gradient
        let pt = (paint_mat() * vec3<f32>(fpos, 1.0)).xy;
        let d = clamp((sdroundrect(pt, extent(), radius()) + feather() * 0.5) / feather(), 0.0, 1.0);
        var color = mix(inner_col(), outer_col(), d);
        color = color * (stroke_alpha * scissor);
        return color;
    } else if ft == 1 {
        // Image
        let pt = (paint_mat() * vec3<f32>(fpos, 1.0)).xy / extent();
        var color = textureSample(tex, tex_sampler, pt);
        let tt = tex_type();
        if tt == 1 {
            color = vec4<f32>(color.xyz * color.w, color.w);
        }
        if tt == 2 {
            color = vec4<f32>(color.x, color.x, color.x, color.x);
        }
        color = color * inner_col();
        color = color * (stroke_alpha * scissor);
        return color;
    } else if ft == 2 {
        // Stencil fill
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if ft == 3 {
        // Textured tris
        var color = textureSample(tex, tex_sampler, ftcoord);
        let tt = tex_type();
        if tt == 1 {
            color = vec4<f32>(color.xyz * color.w, color.w);
        }
        if tt == 2 {
            color = vec4<f32>(color.x, color.x, color.x, color.x);
        }
        color = color * scissor;
        return color * inner_col();
    }

    return vec4<f32>(0.0);
}

// ── Fragment entry points ───────────────────────────────────────────

@fragment
fn fs_main_edge_aa(in: VertexOutput) -> @location(0) vec4<f32> {
    let stroke_alpha = stroke_mask(in.ftcoord);
    if stroke_alpha < stroke_thr() {
        discard;
    }
    return shade(in.ftcoord, in.fpos, stroke_alpha);
}

@fragment
fn fs_main_no_aa(in: VertexOutput) -> @location(0) vec4<f32> {
    return shade(in.ftcoord, in.fpos, 1.0);
}
