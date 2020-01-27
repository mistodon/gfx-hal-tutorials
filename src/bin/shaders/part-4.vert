#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(push_constant) uniform PushConstants {
    mat4 transform;
} push_constants;

layout(location = 0) out vec3 vertex_normal;

void main() {
    vertex_normal = normalize((push_constants.transform * vec4(normal, 0.0)).xyz);
    gl_Position = push_constants.transform * vec4(position, 1.0);
}