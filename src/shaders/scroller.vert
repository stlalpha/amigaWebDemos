attribute vec4 position;
varying vec2 fragCoord;

void main() {
    fragCoord = position.xy * 0.5 + 0.5;  // Convert to UV coordinates
    gl_Position = position;
} 