// Uniforms passed from the application
struct Uniforms {
    mvp_matrix: mat4x4<f32>,  // Model-view-projection matrix
};
@binding(0) @group(0) var<uniform> uniforms: Uniforms;

// Vertex attributes
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    // Instance attributes
    @location(3) start: vec3<f32>,
    @location(4) radius_factor: f32,
    @location(5) end: vec3<f32>,
    @location(6) material_type: u32,
    @location(7) color: vec4<f32>,
};

// Output to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) @interpolate(flat) material_type: u32,
};

// Function to build a transformation matrix for a cylinder from start to end
fn build_cylinder_matrix(start: vec3<f32>, end: vec3<f32>, radius_factor: f32) -> mat4x4<f32> {
    // Calculate direction and length
    let direction = end - start;
    let length = length(direction);

    // Base radius - radius_factor is fabric scale
    let base_radius = 0.04;
    let radius = base_radius * radius_factor;

    // If length is too small, return identity matrix at the midpoint
    if (length < 0.0001) {
        let midpoint = (start + end) * 0.5;
        return mat4x4<f32>(
            vec4<f32>(radius, 0.0, 0.0, 0.0),
            vec4<f32>(0.0, 0.001, 0.0, 0.0), // tiny non-zero length
            vec4<f32>(0.0, 0.0, radius, 0.0),
            vec4<f32>(midpoint.x, midpoint.y, midpoint.z, 1.0)
        );
    }

    // Calculate midpoint for translation
    let midpoint = (start + end) * 0.5;

    // Calculate X basis vector (perpendicular to cylinder axis)
    let y_axis = normalize(direction); // Unit vector along cylinder axis

    // Find X basis vector (perpendicular to Y)
    // Since we want a specific basis, we'll use a consistent approach
    var x_axis: vec3<f32>;
    if (abs(y_axis.y) < 0.999) {
        // Not aligned with global Y, so use global Y to find perpendicular
        x_axis = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), y_axis));
    } else {
        // Aligned with global Y, so use global X to find perpendicular
        x_axis = normalize(cross(vec3<f32>(1.0, 0.0, 0.0), y_axis));
    }

    // Find Z basis vector to complete the basis
    let z_axis = cross(x_axis, y_axis);

    // Build the basis transformation matrix
    return mat4x4<f32>(
        vec4<f32>(x_axis * radius, 0.0),   // Scale X basis by radius
        vec4<f32>(y_axis * length, 0.0),   // Scale Y basis by length
        vec4<f32>(z_axis * radius, 0.0),   // Scale Z basis by radius
        vec4<f32>(midpoint, 1.0)           // Position at midpoint
    );
}

@vertex
fn fabric_vertex(in: VertexInput) -> VertexOutput {
    // Build model matrix on the GPU
    let model_matrix = build_cylinder_matrix(in.start, in.end, in.radius_factor);

    // Transform vertex position to world space
    let world_position = model_matrix * vec4<f32>(in.position, 1.0);

    // Transform normal to world space
    let normal_matrix = mat3x3<f32>(
        normalize(model_matrix[0].xyz),
        normalize(model_matrix[1].xyz),
        normalize(model_matrix[2].xyz)
    );
    let world_normal = normalize(normal_matrix * in.normal);

    var out: VertexOutput;
    out.clip_position = uniforms.mvp_matrix * world_position;
    out.world_position = world_position.xyz;
    out.world_normal = world_normal;
    out.uv = in.uv;
    out.color = in.color;
    out.material_type = in.material_type;

    return out;
}

// Fragment shader
@fragment
fn fabric_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Directional light parameters (daylight-like)
    let light_dir = normalize(vec3<f32>(0.0, 1.0, 0.0));
    let light_color = vec3<f32>(1.0, 0.98, 0.95);

    // Normalized normal vector
    let normal = normalize(in.world_normal);

    // Calculate lighting components
    let ambient = 0.2;
    let diffuse = max(dot(normal, light_dir), 0.0);

    // Calculate view direction (assuming camera at origin)
    // In a full implementation, this would come from a uniform
    let view_dir = normalize(-in.world_position);

    // Calculate half vector for specular lighting
    let half_vec = normalize(light_dir + view_dir);

    // Material-specific properties
    var base_color: vec3<f32>;
    var detail_factor: f32 = 1.0;

    // Different material types (Push=0, Pull=1, Spring=2)
    switch(in.material_type) {
        case 0u: {
            // Push element (aluminum)
            // Add slight metallic tint
            base_color = in.color.rgb * vec3<f32>(0.95, 0.97, 1.0);
            // Add some subtle surface variation
            detail_factor = 1.0 + sin(in.uv.x * 100.0) * 0.01;
            break;
        }
        case 1u: {
            // Pull element (tension cable)
            base_color = in.color.rgb;
            // Add subtle fiber texture
            detail_factor = 0.9 + sin(in.uv.x * 50.0) * 0.1;
            break;
        }
        default: {
            // Spring element
            // Add spring-like pattern
            detail_factor = sin(in.uv.x * 30.0 + in.uv.y * 5.0) * 0.1 + 0.9;
            base_color = in.color.rgb;
            break;
        }
    }

    // Combine lighting components
    let lighting = ambient + diffuse;
    let final_color = base_color * lighting * detail_factor;

    // Apply gamma correction
    let gamma_corrected = pow(final_color, vec3<f32>(1.0/2.2));

    return vec4<f32>(gamma_corrected, in.color.a);
}

struct SurfaceVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn surface_vertex(@location(0) pos: vec4<f32>) -> SurfaceVertexOutput {
    var output: SurfaceVertexOutput;
    output.position = uniforms.mvp_matrix * pos;
    output.world_pos = pos.xyz;
    return output;
}

// Texture bindings for surface
@group(1) @binding(0) var surface_texture: texture_2d<f32>;
@group(1) @binding(1) var surface_sampler: sampler;

@fragment
fn surface_fragment(in: SurfaceVertexOutput) -> @location(0) vec4<f32> {
    // Use world XZ position as UV for tiling (scale for tile size)
    let tile_scale = 0.5; // Adjust for desired tile density
    let uv = in.world_pos.xz * tile_scale;

    // Sample the texture
    let tex_color = textureSample(surface_texture, surface_sampler, uv);

    // Return with some transparency
    return vec4<f32>(tex_color.rgb, 0.8);
}

// Joint marker shader code

// Joint marker vertex input
struct JointVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) instance_position: vec3<f32>,
    @location(4) instance_scale: f32,
    @location(5) instance_color: vec4<f32>,
};

// Joint marker vertex output
struct JointVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn joint_vertex(
    vertex: JointVertexInput,
) -> JointVertexOutput {
    var out: JointVertexOutput;
    
    // Scale the vertex position by the instance scale
    let scaled_position = vertex.position * vertex.instance_scale;
    
    // Translate to the instance position
    let world_position = scaled_position + vertex.instance_position;
    
    // Transform to clip space
    out.clip_position = uniforms.mvp_matrix * vec4<f32>(world_position, 1.0);
    
    // Pass the instance color to the fragment shader
    out.color = vertex.instance_color;
    
    // Pass the normal for lighting calculations
    out.normal = vertex.normal;
    
    return out;
}

@fragment
fn joint_fragment(in: JointVertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting calculation
    let light_direction = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let normal = normalize(in.normal);

    // Calculate diffuse lighting
    let diffuse = max(dot(normal, light_direction), 0.0);

    // Add ambient light
    let ambient = 0.3;
    let lighting = ambient + diffuse * 0.7;

    // Apply lighting to the color
    let final_color = in.color * lighting;

    return final_color;
}

// Ring/Disc shader for attachment point visualization
// Renders flat cylindrical rings oriented perpendicular to push intervals

struct RingVertexInput {
    @location(0) position: vec3<f32>,  // Unit disc position (radius 1, height 1)
    @location(1) normal: vec3<f32>,    // Vertex normal
    @location(2) uv: vec2<f32>,        // Texture coordinates

    // Instance attributes
    @location(3) inst_position: vec3<f32>,  // Ring center position
    @location(4) inst_radius: f32,          // Ring radius
    @location(5) inst_normal: vec3<f32>,    // Ring orientation (push axis)
    @location(6) inst_thickness: f32,       // Ring thickness (height)
    @location(7) inst_color: vec4<f32>,     // Ring color
    @location(8) inst_extension_dir: vec3<f32>,  // Direction to extend toward pull (world space)
    @location(9) inst_extension_len: f32,        // How far to extend (0 = no extension)
};

struct RingVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

// Build a rotation matrix that transforms Y-up to the given normal direction
fn build_ring_rotation_matrix(target_normal: vec3<f32>) -> mat3x3<f32> {
    let y_axis = normalize(target_normal);

    // Find a perpendicular vector for X axis
    var x_axis: vec3<f32>;
    if (abs(y_axis.y) < 0.999) {
        x_axis = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), y_axis));
    } else {
        x_axis = normalize(cross(vec3<f32>(1.0, 0.0, 0.0), y_axis));
    }

    // Z axis completes the basis
    let z_axis = cross(x_axis, y_axis);

    return mat3x3<f32>(x_axis, y_axis, z_axis);
}

@vertex
fn ring_vertex(in: RingVertexInput) -> RingVertexOutput {
    // Scale the unit disc: radius in XZ, thickness in Y
    var scaled_pos = in.position;
    scaled_pos.x *= in.inst_radius;
    scaled_pos.z *= in.inst_radius;
    scaled_pos.y *= in.inst_thickness;

    // Build rotation matrix to orient disc perpendicular to push axis
    let rotation = build_ring_rotation_matrix(in.inst_normal);

    // Rotate the scaled position
    var rotated_pos = rotation * scaled_pos;

    // Translate to instance position
    let world_position = rotated_pos + in.inst_position;

    // Transform normal
    let world_normal = normalize(rotation * in.normal);

    var out: RingVertexOutput;
    out.clip_position = uniforms.mvp_matrix * vec4<f32>(world_position, 1.0);
    out.world_normal = world_normal;
    out.color = in.inst_color;

    return out;
}

@fragment
fn ring_fragment(in: RingVertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting calculation
    let light_direction = normalize(vec3<f32>(0.0, 1.0, 0.0));
    let normal = normalize(in.world_normal);

    // Calculate diffuse lighting (use absolute value for two-sided lighting)
    let diffuse = abs(dot(normal, light_direction));

    // Add ambient light
    let ambient = 0.3;
    let lighting = ambient + diffuse * 0.7;

    // Apply lighting to the color
    let final_color = vec3<f32>(in.color.rgb) * lighting;

    return vec4<f32>(final_color, in.color.a);
}

// ============================================
// Sky shader - starry night background
// ============================================

@group(0) @binding(0) var<uniform> sky_time: f32;

struct SkyVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct SkyVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn sky_vertex(in: SkyVertexInput) -> SkyVertexOutput {
    var out: SkyVertexOutput;
    out.position = vec4<f32>(in.position, 0.999, 1.0);  // Near far plane
    out.uv = in.uv;
    return out;
}

// Hash function for pseudo-random star placement
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Hash function returning vec2 for star properties
fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let p4 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

@fragment
fn sky_fragment(in: SkyVertexOutput) -> @location(0) vec4<f32> {
    // Dark gradient background (darker at top)
    let gradient = mix(
        vec3<f32>(0.01, 0.01, 0.02),  // Bottom: very dark
        vec3<f32>(0.0, 0.0, 0.0),     // Top: black
        in.uv.y
    );

    // Star field - white stars with sparkle
    var star_brightness = 0.0;

    // Multiple layers of stars at different scales
    for (var layer: u32 = 0u; layer < 3u; layer++) {
        let scale = 50.0 + f32(layer) * 80.0;
        let star_uv = in.uv * scale;
        let cell = floor(star_uv);
        let cell_uv = fract(star_uv);

        // Random position within cell
        let star_pos = hash22(cell + f32(layer) * 100.0);

        // Distance from star center
        let dist = length(cell_uv - star_pos);

        // Star brightness (random per star)
        let brightness = hash21(cell + f32(layer) * 200.0);

        // Only show stars above threshold
        if (brightness > 0.92) {
            // Star size varies with brightness
            let star_size = 0.02 + brightness * 0.03;

            // Sparkle effect using time and star position
            let sparkle_phase = hash21(cell + f32(layer) * 300.0) * 6.28318;
            let sparkle_speed = 0.5 + hash21(cell + f32(layer) * 400.0) * 2.0;
            let sparkle = 0.6 + 0.4 * sin(sky_time * sparkle_speed + sparkle_phase);

            // Star intensity with soft falloff
            let intensity = smoothstep(star_size, 0.0, dist) * sparkle;

            star_brightness = star_brightness + intensity * (0.5 + brightness * 0.5);
        }
    }

    let final_color = gradient + vec3<f32>(star_brightness);
    return vec4<f32>(final_color, 1.0);
}
