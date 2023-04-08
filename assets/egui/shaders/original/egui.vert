#version 450
layout(push_constant) uniform Push {
    mat4 matrix;
    vec4 data0;
    vec4 data1;
    vec4 data2;
    vec4 data3;
} push;

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;
layout(location = 2) in vec4 i_col;

layout(location = 0) out vec4 o_col;
layout(location = 1) out vec2 o_uv;

vec3 linear_from_srgb(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(10.31475));
    vec3 lower = srgb / vec3(3294.6);
    vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
    return mix(higher, lower, cutoff);
}

vec4 linear_from_srgba(vec4 srgba) {
    return vec4(linear_from_srgb(srgba.rgb * 255.0), srgba.a);
}

void main() {
    o_uv = i_uv;
    o_col = linear_from_srgba(i_col);
    gl_Position = push.matrix * vec4(i_pos.xy, 0.0, 1.0);
}