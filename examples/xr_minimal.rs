use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::EguiContext;
use bevy_egui::EguiPlugin;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_spatial_egui::SpawnSpatialEguiWindowCommand;
use bevy_suis::debug::SuisDebugGizmosPlugin;
use bevy_suis::window_pointers::SuisWindowPointerPlugin;
use bevy_suis::xr::SuisXrPlugin;
use bevy_suis::xr_controllers::SuisXrControllerPlugin;
use bevy_suis::SuisCorePlugin;
use bevy_mod_openxr::add_xr_plugins;

fn main() -> AppExit {
    App::new()
        .add_plugins(add_xr_plugins(DefaultPlugins))
        .add_plugins(bevy_xr_utils::hand_gizmos::HandGizmosPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins((
            SuisCorePlugin,
            SuisWindowPointerPlugin,
            SuisDebugGizmosPlugin,
            SuisXrPlugin,
            SuisXrControllerPlugin,
        ))
        .add_plugins(bevy_spatial_egui::SpatialEguiPlugin)
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_ui)
        .run()
}

fn draw_ui(mut query: Query<&mut EguiContext, With<MainWindow>>) {
    for mut ctx in &mut query {
        egui::panel::CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Hello, World!");
            if ui.button("Press Me!").clicked() {
                info!("Button Pressed");
            }
        });
    }
}

#[derive(Component)]
struct MainWindow;
fn setup(mut cmds: Commands) {
    let window = cmds.spawn(MainWindow).id();
    cmds.queue(SpawnSpatialEguiWindowCommand {
        target_entity: Some(window),
        position: Vec3::new(0.0, 1.0, -0.5),
        rotation: Quat::IDENTITY,
        resolution: UVec2::splat(512),
        height: 1.0,
        unlit: true,
    });
    cmds.spawn(Camera3d::default())
        .insert(Transform::from_xyz(1.0, 3.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y))
        .insert(PanOrbitCamera::default());
}
