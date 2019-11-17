#version 450

layout(location = 0) in vec2 position;
layout(location = 0) out vec2 uv_coordinates;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    // Flip the uvs because the image we render is upside down.
    // (World Z is up, Image Y is down.)
    uv_coordinates = vec2(position.x / 2.0 + 0.5, position.y / -2.0 + 0.5);
}