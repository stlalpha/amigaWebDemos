precision mediump float;
uniform float time;
uniform vec3 color1;
uniform vec3 color2;

void main() {
    float y = gl_FragCoord.y;
    float wave = sin(y * 0.1 + time) * 0.5 + 0.5;
    vec3 color = mix(color1, color2, wave);
    gl_FragColor = vec4(color, 1.0);
} 