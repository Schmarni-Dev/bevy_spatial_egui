use bevy::{
    math::Vec2,
    render::{mesh::Mesh, render_asset::RenderAssetUsages},
};

// Idea shamelessly copied from StardustXR flatland
pub fn construct_window_mesh(size: Vec2, depth: f32) -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    let top_left_front = [
        (size.x / 2.0) * -1.0,
        (size.y / 2.0) * 1.0,
        (depth / 2.0) * -1.0,
    ];
    let top_right_front = [
        (size.x / 2.0) * 1.0,
        (size.y / 2.0) * 1.0,
        (depth / 2.0) * -1.0,
    ];
    let bottom_left_front = [
        (size.x / 2.0) * -1.0,
        (size.y / 2.0) * -1.0,
        (depth / 2.0) * -1.0,
    ];
    let bottom_right_front = [
        (size.x / 2.0) * 1.0,
        (size.y / 2.0) * -1.0,
        (depth / 2.0) * -1.0,
    ];
    let top_left_back = [
        (size.x / 2.0) * -1.0,
        (size.y / 2.0) * 1.0,
        (depth / 2.0) * 1.0,
    ];
    let top_right_back = [
        (size.x / 2.0) * 1.0,
        (size.y / 2.0) * 1.0,
        (depth / 2.0) * 1.0,
    ];
    let bottom_left_back = [
        (size.x / 2.0) * -1.0,
        (size.y / 2.0) * -1.0,
        (depth / 2.0) * 1.0,
    ];
    let bottom_right_back = [
        (size.x / 2.0) * 1.0,
        (size.y / 2.0) * -1.0,
        (depth / 2.0) * 1.0,
    ];
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            // front
            top_left_front,
            top_right_front,
            bottom_left_front,
            //
            bottom_left_front,
            top_right_front,
            bottom_right_front,
            // back
            top_left_back,
            bottom_left_back,
            top_right_back,
            //
            bottom_left_back,
            bottom_right_back,
            top_right_back,
            // top
            top_left_back,
            top_right_back,
            top_left_front,
            //
            top_right_back,
            top_right_front,
            top_left_front,
            // bottom
            bottom_left_back,
            bottom_left_front,
            bottom_right_back,
            //
            bottom_right_back,
            bottom_left_front,
            bottom_right_front,
            // left
            top_left_back,
            top_left_front,
            bottom_left_back,
            //
            bottom_left_back,
            top_left_front,
            bottom_left_front,
            // right
            top_right_front,
            top_right_back,
            bottom_right_front,
            //
            bottom_right_front,
            top_right_back,
            bottom_right_back,
        ],
    );
    // idk why i have to reverse left/right
    let top_right = [1., 0.];
    let top_left = [0., 0.];
    let bottom_right = [1., 1.];
    let bottom_left = [0., 1.];
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            // front
            top_left,
            top_right,
            bottom_left,
            //
            bottom_left,
            top_right,
            bottom_right,
            // back
            top_left,
            bottom_left,
            top_right,
            //
            bottom_left,
            bottom_right,
            top_right,
            // top
            top_left,
            top_right,
            top_left,
            //
            top_right,
            top_right,
            top_left,
            // bottom
            bottom_left,
            bottom_left,
            bottom_right,
            //
            bottom_right,
            bottom_left,
            bottom_right,
            // left
            top_left,
            top_left,
            bottom_left,
            //
            bottom_left,
            top_left,
            bottom_left,
            // right
            top_right,
            top_right,
            bottom_right,
            //
            bottom_right,
            top_right,
            bottom_right,
        ],
    );

    mesh
}
