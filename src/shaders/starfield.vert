#ifdef GL_ES
precision mediump float;
#endif

attribute vec2 a_position;

void main() {
    // Convert the position from 2D to 3D by adding a z-coordinate of 0.0
    gl_Position = vec4(a_position, 0.0, 1.0);
} 