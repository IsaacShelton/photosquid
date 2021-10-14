#version 140

// Rounded rectangle w/ drop shadow shader
// Modified and improved from https://www.shadertoy.com/view/WtdSDs

// Width and height of quad (not the rectangle) in logical pixels (conversion: rectangle_size + vec2(32.0, 32.0))
// The rectangle itself will be of size (vec2(dimensions.x, dimensions.y * height_scale)) and centered in the middle of the quad
uniform vec2 dimensions;
uniform vec4 rectangle_color;
uniform float height_scale;

in vec2 pass_uvs;
out vec4 out_color;

// from http://www.iquilezles.org/www/articles/distfunctions/distfunctions.htm
float rounded_box_sdf(vec2 center, vec2 size, float radius) {
    return length(max(abs(center) - size + radius, 0.0)) - radius;
}

void main() {
    vec2 coord = pass_uvs * dimensions;
    vec2 padding = vec2(32.0, 32.0);
    vec2 size = vec2(dimensions.x - padding.x, (dimensions.y - padding.y) * height_scale);
    vec2 half_size = size / 2.0;
    
    // the pixel space location of the rectangle.
    vec2 location = padding / 2.0;

    // How soft the edges should be (in pixels). Higher values could be used to simulate a drop shadow.
    float edge_softness  = 1.0;
    
    // The radius of the corners (in pixels).
    float radius = 10.0;
    
    // Calculate distance to edge.   
    float dist = rounded_box_sdf(coord - location - half_size, half_size, radius);
    
    // Smooth the result (free antialiasing).
    float smoothed_alpha =  1.0 - smoothstep(0.0, edge_softness * 2.0, dist);
    
    // Return the resultant shape.
    vec4 quad_color = mix(
        vec4(0.0, 0.0, 0.0, 0.0),        // Background
        vec4(rectangle_color.xyz, smoothed_alpha), // Rectangle Color 0.137,0.153,0.165
        smoothed_alpha
    );
    
    // Apply a drop shadow effect.
    float shadow_softness = 20.0;
    vec2  shadow_offset   = vec2(0.0, -5.0);
    float shadow_distance = rounded_box_sdf(coord - location + shadow_offset - half_size, half_size, radius);
    float shadow_alpha 	  = 1.0 - smoothstep(-shadow_softness, shadow_softness, shadow_distance);
    vec4  shadow_color 	  = vec4(0.01, 0.01, 0.01, shadow_alpha);

    out_color = mix(quad_color, shadow_color, clamp(shadow_alpha - smoothed_alpha, 0.0, 1.0));
}