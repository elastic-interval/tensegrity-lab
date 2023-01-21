struct Uniforms {
    mvpMatrix : mat4x4<f32>,
};
@binding(0) @group(0) var<uniform> uniforms : Uniforms;

struct FabricOutput {
    @builtin(position) Position : vec4<f32>,
    @location(0) vColor : vec4<f32>,
};

@vertex
fn fabric_vertex(@location(0) pos: vec4<f32>, @location(1) color: vec4<f32>) -> FabricOutput {
    var output: FabricOutput;
    output.Position = uniforms.mvpMatrix * pos;
    output.vColor = color;
    return output;
}

@fragment
fn fabric_fragment(in: FabricOutput) -> @location(0) vec4<f32> {
    return in.vColor;
}

struct SurfaceOutput {
    @builtin(position) Position : vec4<f32>,
};

@vertex
fn surface_vertex(@location(0) pos: vec4<f32>) -> SurfaceOutput {
    var output: SurfaceOutput;
    output.Position = uniforms.mvpMatrix * pos;
    return output;
}

@fragment
fn surface_fragment(in: SurfaceOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.5, 0.5, 0.5, 0.5);
}
