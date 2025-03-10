struct Uniforms {
    mvpMatrix : mat4x4<f32>,
};
@binding(0) @group(0) var<uniform> uniforms : Uniforms;

struct VertexInput {
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>
}

struct FragmentInput {
    @builtin(position) position : vec4<f32>,
    @location(0) color : vec4<f32>,
};

@vertex
fn fabric_vertex(input: VertexInput) -> FragmentInput {
    var output: FragmentInput;
    output.position = uniforms.mvpMatrix * input.pos;
    output.color = input.color;
    return output;
}

@fragment
fn fabric_fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    return input.color;
}

struct SurfaceOutput {
    @builtin(position) position : vec4<f32>,
};

@vertex
fn surface_vertex(@location(0) pos: vec4<f32>) -> SurfaceOutput {
    var output: SurfaceOutput;
    output.position = uniforms.mvpMatrix * pos;
    return output;
}

@fragment
fn surface_fragment(in: SurfaceOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.7, 0.7, 0.7, 0.1);
}
