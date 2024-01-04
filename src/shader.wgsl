struct PushConstants {
    flags: i32,
    scale: f32,
    rotation: f32,
    opacity: f32,
    translation_x: i32,
    translation_y: i32,
}
var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) position: vec2<i32>,
    @location(1) uv: vec2<u32>,
    @location(2) color: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct Uniforms {
    ortho: mat4x4<f32>,
    dips_scale: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct Fraction {
    div_by: i32,
    mul_by: i32,
}

fn ratio(raio: u32) -> Fraction {
    var ratio: Fraction;
    ratio.div_by = i32(uniforms.dips_scale >> u32(16));
    ratio.mul_by = i32(uniforms.dips_scale & u32(0xFFFF));
    return ratio;
}

fn ratio_to_f32(ratio: Fraction) -> f32 {
    return f32(ratio.mul_by) / f32(ratio.div_by);
}

fn int_scale(value: i32, ratio: Fraction) -> i32 {
    return value * ratio.mul_by / ratio.div_by;
}

fn dips_to_pixels(value: i32, ratio: Fraction) -> i32 {
    return (int_scale(value, ratio) * i32(96 * 4) + 91439) / i32(182880);
}

fn int_to_rgba(color: u32) -> vec4<f32> {
    let r = color >> u32(24);
    let g = (color >> u32(16)) & u32(0xFF);
    let b = (color >> u32(8)) & u32(0xFF);
    let a = color & u32(0xFF);

    return vec4<f32>(f32(r) / 255.0, f32(g) / 255.0, f32(b) / 255.0, f32(a) / 255.0);
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let flag_dips = u32(1);
    let flag_scale = flag_dips << u32(1);
    let flag_rotation = flag_dips << u32(2);
    let flag_translate = flag_dips << u32(3);

    let flags = u32(pc.flags);

    var dips_scale = ratio(uniforms.dips_scale);
    var outval: VertexOutput;
    var position: vec2<f32>;
    if (flags & flag_dips) != u32(0) {
        position = vec2<f32>(
            f32(dips_to_pixels(input.position.x, dips_scale)),
            f32(dips_to_pixels(input.position.y, dips_scale)),
        );
    } else {
        position = vec2<f32>(
            f32(input.position.x),
            f32(input.position.y),
        );
    }
    if (flags & flag_rotation) != u32(0) {
        var angle_cos = cos(pc.rotation);
        var angle_sin = sin(pc.rotation);
        position = position * mat2x2<f32>(angle_cos, -angle_sin, angle_sin, angle_cos);
    }
    if (flags & flag_scale) != u32(0) {
        position = position * pc.scale;
    }
    if (flags & flag_translate) != u32(0) {
        position = position + vec2<f32>(
            f32(pc.translation_x),
            f32(pc.translation_y)
        );
    }
    outval.position = uniforms.ortho * vec4<f32>(position / 4., 0., 1.0);
    outval.color = int_to_rgba(input.color);
    outval.color.a = pc.opacity * outval.color.a;
    outval.uv = vec2<f32>(input.uv) / vec2<f32>(textureDimensions(r_texture));
    return outval;
}

struct FragmentInput {
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@group(0)
@binding(1)
var r_texture: texture_2d<f32>;
@group(0)
@binding(2)
var r_sampler: sampler;

@fragment
fn fragment(fragment: FragmentInput) -> @location(0) vec4<f32> {
    let flag_textured = u32(1) << u32(4);
    let flag_masked = u32(1) << u32(5);

    var color = fragment.color;

    let flags = u32(pc.flags);
    if (flags & flag_textured) != u32(0) {
        let sample = textureSample(r_texture, r_sampler, fragment.uv / 4.);
        if (flags & flag_masked) != u32(0) {
            return vec4<f32>(color.x, color.y, color.z, sample.x * color.w);
        }

        return sample * color;
    }

    return color;
}
