precision highp float;
varying vec2 fragCoord;
uniform sampler2D iChannel0;
uniform float iTime;
uniform float uScrollSpeed;
uniform vec3 uWaveParams;  // x: amplitude, y: frequency, z: speed
uniform vec3 uColor;

void main() {
    // Convert fragCoord to UV coordinates
    vec2 uv = fragCoord;
    uv.y = 1.0 - uv.y;  // Flip Y to match texture orientation

    // Apply horizontal scrolling
    uv.x = fract(uv.x - iTime * uScrollSpeed);

    // Apply sine wave effect for vertical displacement
    float wave = uWaveParams.x * sin(uv.x * uWaveParams.y + iTime * uWaveParams.z);
    uv.y += wave;

    // Sample the texture
    vec4 texColor = texture2D(iChannel0, uv);

    // Mix the texture color with the uniform color based on alpha
    vec3 finalColor = mix(vec3(0.0), uColor, texColor.a);

    // Output the final color
    gl_FragColor = vec4(finalColor, 1.0);
} 