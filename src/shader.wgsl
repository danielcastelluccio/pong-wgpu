struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] transform: vec3<f32>;
};

struct FragmentInput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn main(in: VertexInput) -> FragmentInput {
    var fragment_input: FragmentInput;
    fragment_input.clip_position = vec4<f32>(in.position + in.transform, 1.0);
    fragment_input.color = in.color;
    return fragment_input;
};

[[stage(fragment)]]
fn main(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    return in.color;
};