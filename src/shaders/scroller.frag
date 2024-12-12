precision highp float;
varying vec2 fragCoord;
uniform sampler2D iChannel0;
uniform float iTime;
uniform float uScrollSpeed;
uniform vec3 uWaveParams;  // x: amplitude, y: frequency, z: speed
uniform vec3 uColor;

void main() {
    vec2 uv = fragCoord;
    uv.y = 1.0 - uv.y;  // Flip Y to match texture orientation

    // Apply horizontal scrolling with clean text
    vec2 textUV = uv;
    textUV.x = fract(1.0 + textUV.x - iTime * 0.2);  // Added 1.0 to start from right
    
    // Keep text straight, only apply wave to vertical position
    float verticalOffset = uWaveParams.x * sin(iTime * uWaveParams.z) * 0.1;
    textUV.y += verticalOffset;

    // Sample the texture
    vec4 texColor = texture2D(iChannel0, textUV);
    texColor.a = 1.0 - texColor.a;  // Invert alpha

    // Mix the texture color with yellow for classic Amiga look
    vec3 finalColor = mix(vec3(0.0), vec3(1.0, 1.0, 0.0), texColor.a);

    gl_FragColor = vec4(finalColor, 1.0);
} 