struct Uniforms {
  surface: vec4<f32>,
  rect: vec4<f32>,
  video: vec4<f32>,
  bg: vec4<f32>,
  mode: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var tex0: texture_2d<f32>;
@group(0) @binding(3) var tex1: texture_2d<f32>;
@group(0) @binding(4) var tex2: texture_2d<f32>;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
  var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
  var o: VsOut;
  o.pos = vec4<f32>(p[vi], 0.0, 1.0);
  o.uv = vec2<f32>(p[vi].x * 0.5 + 0.5, 1.0 - (p[vi].y * 0.5 + 0.5));
  return o;
}

fn yuv_to_rgb(y: f32, cb: f32, cr: f32, full: f32) -> vec3<f32> {
  let yy = mix((y - 16.0 / 255.0) * (255.0 / 219.0), y, full);
  let u2 = mix((cb - 128.0 / 255.0) * (255.0 / 224.0), cb - 128.0 / 255.0, full);
  let v2 = mix((cr - 128.0 / 255.0) * (255.0 / 224.0), cr - 128.0 / 255.0, full);
  let r = yy + 1.5748 * v2;
  let g = yy - 0.1873 * u2 - 0.4681 * v2;
  let b = yy + 1.8556 * u2;
  return clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
  let px = i.uv * u.surface.xy;
  if (px.x < u.rect.x || px.y < u.rect.y || px.x >= u.rect.z || px.y >= u.rect.w) {
    return vec4<f32>(u.bg.rgb, 1.0);
  }
  if (px.x < u.video.x || px.y < u.video.y || px.x >= u.video.z || px.y >= u.video.w) {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
  }
  let vuv = (px - u.video.xy) / (u.video.zw - u.video.xy);
  let m = u32(u.mode.x + 0.5);
  let full = u.mode.y;
  if (m == 2u) {
    let y = textureSample(tex0, samp, vuv).r;
    let uv = textureSample(tex1, samp, vuv).rg;
    return vec4<f32>(yuv_to_rgb(y, uv.x, uv.y, full), 1.0);
  } else if (m == 3u) {
    let y = textureSample(tex0, samp, vuv).r;
    let cb = textureSample(tex1, samp, vuv).r;
    let cr = textureSample(tex2, samp, vuv).r;
    return vec4<f32>(yuv_to_rgb(y, cb, cr, full), 1.0);
  }
  return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
