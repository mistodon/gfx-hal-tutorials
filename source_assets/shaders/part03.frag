#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 varying_color;

layout(location = 0) out vec4 target;

void main() {
    target = varying_color;
}

