#version 140

uniform vec2 dimensions;  // Width and height of color picker box in logical pixels
uniform vec2 point;       // Selection point in UV coords
uniform float saturation; // HSV Saturation

in vec2 pass_uvs;
out vec4 out_color;

// https://stackoverflow.com/questions/15095909/from-rgb-to-hsv-in-opengl-glsl
vec3 hsv2rgb(vec3 c) {
    // All components are in the range 0..1, including hue.
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

// Custom function for strength of fragment near selection point
// where 'x' is [0..1] representing the percentage of the total radius
float f(float x) {
    const float bumpers = 0.2;
    
    if (x < bumpers) {
        return smoothstep(0.0, 1.0, x * (1.0 / bumpers));
    } else if (x > 1.0 - bumpers) {
        return 1.0 - smoothstep(0.0, 1.0, (x - (1.0 - bumpers)) * (1.0 / bumpers));
    } else {
        return 1.0;
    }
}

void main() {
    vec2 uv = vec2(pass_uvs.x, 1.0 - pass_uvs.y);
    vec3 col = hsv2rgb(vec3(uv.x, saturation, uv.y));
    vec2 point = vec2(point.x, 1.0 - point.y);
    float x_diff = (uv.x - point.x) * dimensions.x;
    float y_diff = (uv.y - point.y) * dimensions.y;
    
    float size = 0.5;
    float d2 = x_diff * x_diff + y_diff * y_diff;
    
    if (d2 < 14.0 * 14.0 * size * size && d2 > 6.0 * 6.0 * size * size) {
        float d = sqrt(d2);
        
        // [0.0, 1.0]
        float x = (d2 - 36.0 * size * size) / (196.0 * size * size - 36.0 * size * size);
        x = smoothstep(0.0, 1.0, x);
        
        float v = f(x);
        col = vec3(v) + (1.0 - v) * col;
    }
    
    out_color = vec4(col, 1.0);
}
