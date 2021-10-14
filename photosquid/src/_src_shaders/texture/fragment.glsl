#version 140

uniform sampler2D texture_sampler;

in vec2 pass_uvs;
out vec4 out_color;

void main() {
    out_color = texture(texture_sampler, pass_uvs);
}
