// Bonsai Creator — 3D Gaussian Splatting WebGPU shader.
//
// Compute pass: radix_sort_histogram — compute view-space depth for each
// Gaussian and initialise sorted_indices with identity permutation.
//
// Vertex pass: vs_main — project sorted Gaussian splat centres onto the
// screen, emit a quad [−1,+1]² in UV space, evaluate SH colour.
//
// Fragment pass: fs_main — evaluate the 2D Gaussian kernel on the quad,
// discard outside the unit circle, output pre-multiplied alpha.

// ── Struct definitions ────────────────────────────────────────────────────────

struct Splat {
    position : vec3f,
    _pad0    : f32,
    scale    : vec3f,
    _pad1    : f32,
    rotation : vec4f,
    opacity  : f32,
    _pad2    : vec3f,
    sh       : array<f32, 48>,
};

struct CameraUniform {
    view_proj  : mat4x4f,
    camera_pos : vec3f,
    _padding   : f32,
};

// ── Bindings ──────────────────────────────────────────────────────────────────

@group(0) @binding(0) var<uniform>            camera        : CameraUniform;
@group(0) @binding(1) var<storage, read>      splats        : array<Splat>;
@group(0) @binding(2) var<storage, read_write> sorted_indices : array<u32>;
@group(0) @binding(3) var<storage, read_write> depths        : array<f32>;

// ── Compute: depth fill ───────────────────────────────────────────────────────

@compute @workgroup_size(256)
fn radix_sort_histogram(@builtin(global_invocation_id) gid : vec3u) {
    let i = gid.x;
    if i >= arrayLength(&splats) { return; }

    // View-space depth along −Z.
    let rel   = splats[i].position - camera.camera_pos;
    let depth = -(camera.view_proj[2][0] * rel.x
               + camera.view_proj[2][1] * rel.y
               + camera.view_proj[2][2] * rel.z);

    depths[i]        = depth;
    sorted_indices[i] = i;   // identity permutation; a multi-pass sort follows in a real impl
}

// ── SH evaluation (degree 3, RGB) ────────────────────────────────────────────

fn sh_eval(dir : vec3f, sh : array<f32, 48>) -> vec3f {
    // SH basis coefficients.  Layout: [dc₀…dc15] × 3 channels.
    // Channel R = sh[0..15], G = sh[16..31], B = sh[32..47].
    var rgb = vec3f(sh[0], sh[16], sh[32]) * 0.2820948; // DC

    // L=1
    rgb += vec3f(sh[1],  sh[17], sh[33]) * 0.4886025 * dir.y;
    rgb += vec3f(sh[2],  sh[18], sh[34]) * 0.4886025 * dir.z;
    rgb += vec3f(sh[3],  sh[19], sh[35]) * 0.4886025 * dir.x;

    // L=2
    let xx = dir.x * dir.x;
    let yy = dir.y * dir.y;
    let zz = dir.z * dir.z;
    let xy = dir.x * dir.y;
    let yz = dir.y * dir.z;
    let xz = dir.x * dir.z;

    rgb += vec3f(sh[4],  sh[20], sh[36]) * 1.0925484 * xy;
    rgb += vec3f(sh[5],  sh[21], sh[37]) * 1.0925484 * yz;
    rgb += vec3f(sh[6],  sh[22], sh[38]) * 0.3153916 * (3.0 * zz - 1.0);
    rgb += vec3f(sh[7],  sh[23], sh[39]) * 1.0925484 * xz;
    rgb += vec3f(sh[8],  sh[24], sh[40]) * 0.5462742 * (xx - yy);

    // L=3
    rgb += vec3f(sh[9],  sh[25], sh[41]) * 0.5900436 * dir.y * (3.0 * xx - yy);
    rgb += vec3f(sh[10], sh[26], sh[42]) * 2.8906114 * xy * dir.z;
    rgb += vec3f(sh[11], sh[27], sh[43]) * 0.4570458 * dir.y * (5.0 * zz - 1.0);
    rgb += vec3f(sh[12], sh[28], sh[44]) * 0.3731763 * dir.z * (5.0 * zz - 3.0);
    rgb += vec3f(sh[13], sh[29], sh[45]) * 0.4570458 * dir.x * (5.0 * zz - 1.0);
    rgb += vec3f(sh[14], sh[30], sh[46]) * 1.4453057 * dir.z * (xx - yy);
    rgb += vec3f(sh[15], sh[31], sh[47]) * 0.5900436 * dir.x * (xx - 3.0 * yy);

    return max(rgb + 0.5, vec3f(0.0));
}

// ── Vertex shader ─────────────────────────────────────────────────────────────

struct VertexOutput {
    @builtin(position)              clip_pos  : vec4f,
    @location(0)                    color     : vec4f,
    @location(1)                    uv        : vec2f,
    @location(2) @interpolate(flat) splat_idx : u32,
};

// Quad vertex positions in local 2D splat space.
const QUAD : array<vec2f, 4> = array<vec2f, 4>(
    vec2f(-1.0, -1.0),
    vec2f( 1.0, -1.0),
    vec2f(-1.0,  1.0),
    vec2f( 1.0,  1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index)   vid : u32,
    @builtin(instance_index) sid : u32,
) -> VertexOutput {
    let idx    = sorted_indices[sid];
    let splat  = splats[idx];

    let clip_centre = camera.view_proj * vec4f(splat.position, 1.0);
    let uv          = QUAD[vid];

    // Project splat scale to screen space (approximate: use longest axis).
    let screen_size = max(splat.scale.x, max(splat.scale.y, splat.scale.z));

    var out       : VertexOutput;
    out.clip_pos  = vec4f(
        clip_centre.x / clip_centre.w + uv.x * screen_size * 0.01,
        clip_centre.y / clip_centre.w + uv.y * screen_size * 0.01,
        clip_centre.z / clip_centre.w,
        1.0
    );
    out.uv        = uv;
    out.splat_idx = idx;

    let view_dir = normalize(camera.camera_pos - splat.position);
    let rgb      = sh_eval(view_dir, splat.sh);
    out.color    = vec4f(rgb, splat.opacity);

    return out;
}

// ── Fragment shader ───────────────────────────────────────────────────────────

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4f {
    let d2 = dot(in.uv, in.uv);
    if d2 > 1.0 { discard; }

    // 2D Gaussian kernel: exp(−d² * 4.0) — tuned for smooth falloff.
    let kernel_alpha = exp(-d2 * 4.0) * in.color.a;
    return vec4f(in.color.rgb * kernel_alpha, kernel_alpha);
}
