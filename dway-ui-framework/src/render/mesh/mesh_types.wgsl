#define_import_path dway_ui_framework::render::mesh_types

struct UiMesh2d {
    // Affine 4x3 matrix transposed to 3x4
    // Use bevy_render::maths::affine_to_square to unpack
    model: mat3x4<f32>,
    // 3x3 matrix packed in mat2x4 and f32 as:
    // [0].xyz, [1].x,
    // [1].yz, [2].xy
    // [2].z
    // Use bevy_render::maths::mat2x4_f32_to_mat3x3_unpack to unpack
    inverse_transpose_model_a: mat2x4<f32>,
    inverse_transpose_model_b: f32,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
    rect: mat2x2<f32>,
};

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(1) @binding(0) var<uniform> mesh: array<UiMesh2d, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(1) @binding(0) var<storage> mesh: array<UiMesh2d>;
#endif // PER_OBJECT_BUFFER_BATCH_SIZE
