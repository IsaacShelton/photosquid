#version 140

in vec2 position;
in vec2 uvs;
out vec2 pass_uvs;

uniform mat4 transformation;
uniform mat4 view;
uniform mat4 projection;

void main() {
    pass_uvs = uvs;
    gl_Position = projection * view * transformation * vec4(position, 0.0, 1.0);
}
