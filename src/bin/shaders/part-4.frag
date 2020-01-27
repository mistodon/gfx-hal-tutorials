#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 vertex_normal;

layout(location = 0) out vec4 fragment_color;

layout(binding = 0) uniform UniformBlock {
    vec4 ambient_light;
    vec4 light_direction;
    vec4 light_color;
} uniform_block;

void main() {
    vec3 to_light = normalize((-uniform_block.light_direction).xyz);

    float light_contribution = clamp(dot(to_light, vertex_normal), 0.0, 1.0);

    vec4 lighting = uniform_block.ambient_light
        + (uniform_block.light_color * light_contribution);

    fragment_color = vec4(lighting.rgb, 1.0);
}