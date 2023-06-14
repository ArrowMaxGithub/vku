#version 450
layout(push_constant) uniform Push {
    mat4 ui_to_ndc;
    uvec4 unused_vec_0;
    vec4 unused_vec_1;
    vec4 unused_vec_2;
    vec4 unused_vec_3;
} push;

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;
layout(location = 2) in vec4 i_col;

layout(location = 0) out vec4 o_col;
layout(location = 1) out vec2 o_uv;

void main() {
    o_uv = i_uv;
    o_col = i_col;
    gl_Position = push.ui_to_ndc * vec4(i_pos.xy, 0.0, 1.0);
}