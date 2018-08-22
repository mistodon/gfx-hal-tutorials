#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;
layout(location = 2) in vec2 uv;

layout(location = 0) out vec4 varying_color;
layout(location = 1) out vec2 varying_uv;

layout(binding = 0) uniform UniformBlock {
    mat4 projection;
} uniform_block;

layout(push_constant) uniform PushConstants {
    vec4 tint;
    vec3 position;
} push_constants;

void main() {
    varying_color = color * push_constants.tint;
    varying_uv = uv;
    gl_Position = uniform_block.projection
        * vec4(position + push_constants.position, 1.0);
}
