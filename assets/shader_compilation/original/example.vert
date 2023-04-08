#version 450
#include "../../../assets/shader_compilation/original/example.glsl" // relative path from calling exe inside /target to assets folder

layout(location = 0) in vec4 i_pos_size;
layout(location = 1) in vec4 i_col;

layout(location = 0) out vec4 o_col;

void main() {
    o_col = i_col;
    gl_Position = vec4(i_pos_size.xyz, 1.0);
    gl_PointSize  = i_pos_size.w;
}