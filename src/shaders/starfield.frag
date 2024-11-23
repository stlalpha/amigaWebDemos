#ifdef GL_ES
precision mediump float;
#endif

// Shadertoy uniforms mapped to WebGL uniforms
uniform vec3 iResolution;
uniform float iTime;
uniform float iTimeDelta;
uniform float iFrameRate;
uniform int iFrame;
uniform vec4 iMouse;

// divisions of grid
const float repeats = 30.;

// number of layers
const float layers = 21.;

// star colours
const vec3 blue = vec3(51.,64.,195.)/255.;
const vec3 cyan = vec3(117.,250.,254.)/255.;
const vec3 white = vec3(255.,255.,255.)/255.;
const vec3 yellow = vec3(251.,245.,44.)/255.;
const vec3 red = vec3(247,2.,20.)/255.;

// spectrum function
vec3 spectrum(vec2 pos){
    pos.x *= 4.;
    vec3 outCol = vec3(0);
    if( pos.x > 0.){
        outCol = mix(blue, cyan, fract(pos.x));
    }
    if( pos.x > 1.){
        outCol = mix(cyan, white, fract(pos.x));
    }
    if( pos.x > 2.){
        outCol = mix(white, yellow, fract(pos.x));
    }
    if( pos.x > 3.){
        outCol = mix(yellow, red, fract(pos.x));
    }
    
    return 1.-(pos.y * (1.-outCol));
}

float N21(vec2 p){
    p = fract(p*vec2(233.34, 851.73));
    p+= dot(p, p+23.45);
    return fract(p.x*p.y);
}

vec2 N22 (vec2 p){
    float n = N21(p);
    return vec2(n, N21(p+n));
}

mat2 scale(vec2 _scale){
    return mat2(_scale.x,0.0,
                0.0,_scale.y);
}

float noise (in vec2 st) {
    vec2 i = floor(st);
    vec2 f = fract(st);

    float a = N21(i);
    float b = N21(i + vec2(1.0, 0.0));
    float c = N21(i + vec2(0.0, 1.0));
    float d = N21(i + vec2(1.0, 1.0));

    vec2 u = f*f*(3.0-2.0*f);

    return mix(a, b, u.x) +
            (c - a)* u.y * (1.0 - u.x) +
            (d - b) * u.x * u.y;
}

float perlin2(vec2 uv, int octaves, float pscale){
    float col = 1.;
    float initScale = 4.;  
    
    // Fixed iteration count for WebGL 1.0 compatibility
    for (int l = 0; l < 4; l++) { // Changed from variable to constant
        if (l >= octaves) break;  // Early exit if we reach the requested octaves
        float val = noise(uv*initScale);
        if (col <= 0.01){
            col = 0.;
            break;
        }
        val -= 0.01;
        val *= 0.5;
        col *= val;
        initScale *= pscale;
    }
    return col;
}

vec3 stars(vec2 uv, float offset){
    float timeScale = -(iTime + offset) / layers;
    float trans = fract(timeScale);
    float newRnd = floor(timeScale);
    
    vec3 col = vec3(0.);
   
    uv -= vec2(0.5);
    uv = scale(vec2(trans)) * uv;
    uv += vec2(0.5);
    
    uv.x *= iResolution.x / iResolution.y;
    
    float colR = N21(vec2(offset+newRnd));
    float colB = N21(vec2(offset+newRnd*123.));
    
    if (mod(offset,3.) == 0.){
        float perl = perlin2(uv+offset+newRnd,3,2.);
        col += vec3(perl*colR,perl*0.1,perl*colB);
    }
    
    uv *= repeats;
    
    vec2 ipos = floor(uv);
    uv = fract(uv);
    
    vec2 rndXY = N22(newRnd + ipos*(offset+1.))*0.9+0.05;
    float rndSize = N21(ipos)*100.+200.;
    
    vec2 j = (rndXY - uv)*rndSize;
    float sparkle = 1./dot(j,j);
    
    col += spectrum(fract(rndXY*newRnd*ipos)) * vec3(sparkle);
    
    col *= smoothstep(1.,0.8,trans);    
    col *= smoothstep(0.,0.1,trans);
    return col;
}

void main() {
    vec2 fragCoord = gl_FragCoord.xy;
    vec3 col = vec3(0.);
    
    for (float i = 0.; i < layers; i++) {
        col += stars(fragCoord/iResolution.xy, i);
    }

    gl_FragColor = vec4(col, 1.0);
} 