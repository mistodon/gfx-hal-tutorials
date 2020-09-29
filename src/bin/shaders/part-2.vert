#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(push_constant) uniform PushConstants {
    vec4 color;
    vec2 pos;
    vec2 scale;
} push_constants;

layout(location = 0) out vec4 vertex_color;

vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),
    vec2(-0.5, 0.5),
    vec2(0.5, 0.5)
);

void main() {
    vec2 pos = positions[gl_VertexIndex] * push_constants.scale;
    vertex_color = push_constants.color;
    gl_Position = vec4((pos + push_constants.pos), 0.0, 1.0);
}