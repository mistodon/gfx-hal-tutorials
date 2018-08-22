#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 varying_color;
layout(location = 1) in vec2 varying_uv;

layout(location = 0) out vec4 target;

layout(set = 0, binding = 1) uniform texture2D colormap;
layout(set = 0, binding = 2) uniform sampler colorsampler;

void main() {
    target = varying_color * texture(sampler2D(colormap, colorsampler), varying_uv);
}

