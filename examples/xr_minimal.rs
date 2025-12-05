use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::EguiContext;
use bevy_egui::EguiPlugin;
use bevy_mod_openxr::add_xr_plugins;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_spatial_egui::SpawnSpatialEguiWindowCommand;
use bevy_suis::debug::SuisDebugGizmosPlugin;
use bevy_suis::default_input_methods::xr_controllers::{
    default_bindings::SuisXrControllerDefaultBindingsPlugin,
    interaction_profiles::{SupportedInteractionProfile, SupportedInteractionProfiles},
};
use bevy_suis::default_input_methods::SuisBundledInputMethodPlugins;
use bevy_suis::SuisPlugins;

fn main() -> AppExit {
    App::new()
        .add_plugins(add_xr_plugins(DefaultPlugins))
        .add_plugins(bevy_mod_xr::hand_debug_gizmos::HandGizmosPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins((
            SuisPlugins,
            SuisBundledInputMethodPlugins,
            SuisDebugGizmosPlugin,
            SuisXrControllerDefaultBindingsPlugin {
                supported_interaction_profiles: SupportedInteractionProfiles(HashSet::from_iter([
                    SupportedInteractionProfile::OculusTouch,
                ])),
            },
        ))
        .add_plugins(bevy_spatial_egui::SpatialEguiPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
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
        position: Vec3::new(0.0, 1.0, -0.2),
        rotation: Quat::IDENTITY,
        resolution: UVec2::splat(512),
        height: 1.0,
        unlit: true,
    });
    cmds.spawn((
        PanOrbitCamera::default(),
        Transform::from_xyz(0.5, 1.5, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
