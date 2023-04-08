#version 450

layout(location = 0) in vec4 o_color;
layout(location = 1) in vec2 i_uv;

layout(binding = 0, set = 0) uniform sampler2D fonts_sampler;

layout(location = 0) out vec4 final_color;

void main() {
    vec4 texture_in_gamma = texture(fonts_sampler, i_uv);
    final_color = o_color * texture_in_gamma;
}