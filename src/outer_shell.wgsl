struct Indices {
    indices: array<u16>,
}

@binding(0) @group(0) var<storage, read> points: array<vec3<f32>>;
@binding(1) @group(0) var<storage, read_write> output: Indices;

const CONNECTIONS_PER_POINT = 3;

// TODO: currently this shader will create double indices

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) GlobalInvocationID : vec3<u32>) {
    var index = GlobalInvocationID.x;

    var Vpos = points[index];
    var connections = 0;

    for (var i = 0u; i < arrayLength(&points) && connections < 3; i++) {
        if (i == index) {
            continue;
        }

        var pos = points[i];

        if(distance(pos, Vpos) < 0.2) {
            output[CONNECTIONS_PER_POINT * 2 * index + connections * 2] = index;
            output[CONNECTIONS_PER_POINT * 2 * index + connections * 2 + 1] = index;
        }
    }

}