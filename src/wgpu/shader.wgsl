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
    @location(4) material_type: u32,
};

// Function to build a transformation matrix for a cylinder from start to end
fn build_cylinder_matrix(start: vec3<f32>, end: vec3<f32>, radius_factor: f32) -> mat4x4<f32> {
    // Calculate direction and length
    let direction = end - start;
    let length = length(direction);

    // Base radius - can be adjusted globally here
    let base_radius = 0.03;
    let radius = base_radius * radius_factor;

    // If length is too small, return identity matrix
    if (length < 0.0001) {
        return mat4x4<f32>(
            vec4<f32>(1.0, 0.0, 0.0, 0.0),
            vec4<f32>(0.0, 1.0, 0.0, 0.0),
            vec4<f32>(0.0, 0.0, 1.0, 0.0),
            vec4<f32>(0.0, 0.0, 0.0, 1.0)
        );
    }

    // Calculate midpoint for translation
    let midpoint = start + direction * 0.5;

    // Normalize direction
    let dir = direction / length;

    // Default cylinder orientation is along Y axis
    let y_axis = vec3<f32>(0.0, 1.0, 0.0);

    // Find rotation axis using cross product
    var rotation_axis = cross(y_axis, dir);
    var rotation_axis_length = length(rotation_axis);

    // Handle case where direction is parallel to Y axis
    var rotation_matrix: mat3x3<f32>;
    if (rotation_axis_length < 0.001) {
        // Check if pointing up or down
        if (dir.y > 0.0) {
            // Same as y_axis, use identity rotation
            rotation_matrix = mat3x3<f32>(
                vec3<f32>(1.0, 0.0, 0.0),
                vec3<f32>(0.0, 1.0, 0.0),
                vec3<f32>(0.0, 0.0, 1.0)
            );
        } else {
            // Opposite to y_axis, rotate 180° around X
            rotation_matrix = mat3x3<f32>(
                vec3<f32>(1.0, 0.0, 0.0),
                vec3<f32>(0.0, -1.0, 0.0),
                vec3<f32>(0.0, 0.0, -1.0)
            );
        }
    } else {
        // Normal case: build rotation matrix from axis and angle
        rotation_axis = rotation_axis / rotation_axis_length;
        let cos_angle = dot(y_axis, dir);
        let angle = acos(clamp(cos_angle, -1.0, 1.0));
        let sin_angle = sin(angle);

        // Build rotation matrix using Rodrigues' rotation formula
        let K = mat3x3<f32>(
            vec3<f32>(0.0, -rotation_axis.z, rotation_axis.y),
            vec3<f32>(rotation_axis.z, 0.0, -rotation_axis.x),
            vec3<f32>(-rotation_axis.y, rotation_axis.x, 0.0)
        );

        let I = mat3x3<f32>(
            vec3<f32>(1.0, 0.0, 0.0),
            vec3<f32>(0.0, 1.0, 0.0),
            vec3<f32>(0.0, 0.0, 1.0)
        );

        rotation_matrix = I + sin_angle * K + (1.0 - cos_angle) * (K * K);
    }

    // Build the complete 4x4 transformation matrix
    // Scale by radius in X and Z, by length in Y
    // Rotate to align with direction
    // Translate to midpoint
    let scale_matrix = mat4x4<f32>(
        vec4<f32>(radius, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, length, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, radius, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    let rotation_matrix_4x4 = mat4x4<f32>(
        vec4<f32>(rotation_matrix[0], 0.0),
        vec4<f32>(rotation_matrix[1], 0.0),
        vec4<f32>(rotation_matrix[2], 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    let translation_matrix = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(midpoint, 1.0)
    );

    return translation_matrix * rotation_matrix_4x4 * scale_matrix;
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
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
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
    var specular_power: f32;
    var specular_intensity: f32;
    var base_color: vec3<f32>;
    var detail_factor: f32 = 1.0;

    // Different material types (Push=0, Pull=1, Spring=2)
    switch(in.material_type) {
        case 0u: {
            // Push element (aluminum)
            specular_power = 60.0;
            specular_intensity = 0.7;
            // Add slight metallic tint
            base_color = in.color.rgb * vec3<f32>(0.95, 0.97, 1.0);
            // Add some subtle surface variation
            detail_factor = 1.0 + sin(in.uv.x * 100.0) * 0.02;
            break;
        }
        case 1u: {
            // Pull element (tension cable)
            specular_power = 10.0;
            specular_intensity = 0.1;
            // Add subtle fiber texture
            detail_factor = 0.9 + sin(in.uv.x * 50.0) * 0.1;
            base_color = in.color.rgb;
            break;
        }
        default: {
            // Spring element
            specular_power = 30.0;
            specular_intensity = 0.3;
            // Add spring-like pattern
            detail_factor = sin(in.uv.x * 30.0 + in.uv.y * 5.0) * 0.1 + 0.9;
            base_color = in.color.rgb;
            break;
        }
    }

    // Calculate specular component
    let specular = pow(max(dot(normal, half_vec), 0.0), specular_power) * specular_intensity;

    // Combine lighting components
    let lighting = ambient + diffuse;
    let final_color = base_color * lighting * detail_factor + specular * light_color;

    // Apply gamma correction
    let gamma_corrected = pow(final_color, vec3<f32>(1.0/2.2));

    return vec4<f32>(gamma_corrected, in.color.a);
}

struct SurfaceOutput {
    @builtin(position) position : vec4<f32>,
};

@vertex
fn surface_vertex(@location(0) pos: vec4<f32>) -> SurfaceOutput {
    var output: SurfaceOutput;
    output.position = uniforms.mvp_matrix * pos;
    return output;
}

@fragment
fn surface_fragment(in: SurfaceOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.7, 0.7, 0.7, 0.1);
}
