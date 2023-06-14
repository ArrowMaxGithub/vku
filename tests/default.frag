#version 450
layout(location = 0) in vec4 i_col;
layout(location = 1) in vec2 i_uv;
layout(location = 0) out vec4 o_col;

layout(binding = 0, set = 0) uniform sampler2D tex_sampler;

void main() {
    o_col = i_col * texture(tex_sampler, i_uv);
}