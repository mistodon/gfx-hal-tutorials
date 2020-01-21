#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) out vec4 target;

void main() {
    target = vec4(0.5, 0.5, 1.0, 1.0);
}
