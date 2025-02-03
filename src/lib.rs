pub mod window_mesh;

use std::mem;

use bevy::{
    color::palettes::css,
    ecs::{entity::EntityHashMap, world::Command},
    prelude::*,
    render::render_resource::{Extent3d, LoadOp, TextureUsages},
    window::PrimaryWindow,
};
use bevy_egui::{
    egui::{self, Pos2}, EguiContext, EguiInput, EguiPreUpdateSet, EguiRenderToImage
};
use bevy_suis::{
    window_pointers::MouseInputMethodData, xr::HandInputMethodData,
    xr_controllers::XrControllerInputMethodData, CaptureContext, Field, InputHandler,
    InputHandlerCaptures, InputHandlingContext, PointerInputMethod,
};
use window_mesh::construct_window_mesh;

pub struct SpatialEguiPlugin;

impl Plugin for SpatialEguiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            bevy_suis::pipe_input_ctx::<()>.pipe(update_windows),
        );
        app.add_systems(
            PreUpdate,
            forward_egui_events
                .after(EguiPreUpdateSet::ProcessInput)
                .before(EguiPreUpdateSet::BeginPass),
        );
    }
}
#[derive(Component, Clone, Copy, Debug)]
pub struct ImmovableSpatialEguiWindow;

fn forward_egui_events(
    mut query: Query<&mut EguiInput, With<SpatialEguiWindow>>,
    window_query: Query<&EguiInput, (With<PrimaryWindow>, Without<SpatialEguiWindow>)>,
) {
    let Ok(primary_input) = window_query.get_single() else {
        //warn!("Unable to find one Primary Window!");
        return;
    };

    let events = primary_input.events.iter().filter_map(|e| match e {
        egui::Event::Copy => Some(e.clone()),
        egui::Event::Cut => Some(e.clone()),
        egui::Event::Paste(_) => Some(e.clone()),
        egui::Event::Text(_) => Some(e.clone()),
        egui::Event::Key {
            key: _,
            physical_key: _,
            pressed: _,
            repeat: _,
            modifiers: _,
        } => Some(e.clone()),
        _ => None,
    });

    for mut egui_input in query.iter_mut() {
        egui_input.events.extend(events.clone());
    }
}

#[derive(Clone, Copy, Component)]
struct GrabbedEguiWindow {
    method_relative_transform: Transform,
}

fn update_windows(
    ctxs: In<Vec<InputHandlingContext>>,
    images: Res<Assets<Image>>,
    mut windows: Query<
        (
            &InputHandlerCaptures,
            &SpatialEguiWindowPhysicalSize,
            &mut EguiInput,
            &mut EguiContext,
            &EguiRenderToImage,
            Option<&mut GrabbedEguiWindow>,
            &mut Transform,
            Option<&Parent>,
            Has<ImmovableSpatialEguiWindow>,
        ),
        With<SpatialEguiWindow>,
    >,
    methods: Query<(
        &GlobalTransform,
        Option<&XrControllerInputMethodData>,
        Option<&HandInputMethodData>,
        Option<&MouseInputMethodData>,
        Has<PointerInputMethod>,
    )>,
    mut state: Local<EntityHashMap<EntityHashMap<InputState>>>,
    gt_query: Query<&GlobalTransform>,
    time: Res<Time>,
    mut cmds: Commands,
) {
    for ctx in ctxs.iter() {
        let Ok((
            handler,
            phys_size,
            mut egui_input,
            mut egui_ctx,
            image_handle,
            mut grabbed,
            mut window_transform,
            parent,
            immovable,
        )) = windows.get_mut(ctx.handler)
        else {
            continue;
        };
        if handler.captured_methods.is_empty() {
            egui_input.events.push(egui::Event::PointerGone);
        }
        let resolution = images.get(&image_handle.handle).unwrap().size_f32();
        let mut next_states = EntityHashMap::<InputState>::default();
        for (method_ctx, (method_gt, xr_controller_data, xr_hand_data, mouse_data, is_pointer)) in
            ctx.methods
                .iter()
                .filter_map(|ctx| methods.get(ctx.input_method).map(|v| (ctx, v)).ok())
        {
            let mut current_state = InputState::default();
            if method_ctx
                .closest_point
                .distance(method_ctx.input_method_location.translation)
                <= f32::EPSILON
                && !is_pointer
            {
                current_state.click = true;
            }
            if let Some(controller) = xr_controller_data {
                current_state.click |= controller.trigger_pulled;
                current_state.grab |= controller.squeezed;
                current_state.continuous_scroll +=
                    controller.stick_pos * time.delta_secs() * 1000.;
            }
            if let Some(hand) = xr_hand_data {
                let hand = hand.get_in_relative_space(&ctx.handler_location);
                current_state.click |= hand.index.tip.pos.distance(hand.thumb.tip.pos)
                    > (0.002 + hand.index.tip.radius + hand.thumb.tip.radius)
            }
            if let Some(mouse) = mouse_data {
                current_state.click |= mouse.left_button.pressed;
                current_state.grab |= mouse.right_button.pressed;
                current_state.discrete_scroll += mouse.discrete_scroll;
                current_state.continuous_scroll += mouse.continuous_scroll;
            }
            let last_state = state
                .entry(ctx.handler)
                .or_default()
                .remove(&method_ctx.input_method)
                .unwrap_or_default();
            if (!current_state.grab) && last_state.grab {
                cmds.entity(ctx.handler).remove::<GrabbedEguiWindow>();
            }
            if current_state.grab && (!last_state.grab) && !immovable {
                cmds.entity(ctx.handler).insert(GrabbedEguiWindow {
                    method_relative_transform: Transform::from_matrix(
                        method_gt.compute_matrix().inverse()
                            * ctx.handler_location.compute_matrix(),
                    ),
                });
            }
            if let Some(grabbed) = grabbed.as_mut() {
                let offset_matrix = parent
                    .and_then(|e| gt_query.get(e.get()).ok())
                    .unwrap_or(&GlobalTransform::IDENTITY);

                grabbed.method_relative_transform.translation.z +=
                    current_state.continuous_scroll.y / 500.0;
                grabbed.method_relative_transform.translation.z +=
                    current_state.discrete_scroll.y / 10.0;

                *window_transform = Transform::from_matrix(
                    method_gt
                        .mul_transform(grabbed.method_relative_transform)
                        .compute_matrix()
                        * offset_matrix.compute_matrix().inverse(),
                );
            }
            if grabbed.is_none() {
                let uv = ((method_ctx.closest_point.xy() / phys_size.0.xy()) * -1.) + 0.5;
                let pos = egui::Pos2 {
                    x: (uv.x * resolution.x) / egui_ctx.get_mut().pixels_per_point(),
                    y: (uv.y * resolution.y) / egui_ctx.get_mut().pixels_per_point(),
                };
                egui_input.events.push(egui::Event::PointerMoved(pos));
                if current_state.click && !last_state.click {
                    egui_input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                if !current_state.click && last_state.click {
                    egui_input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                if current_state.discrete_scroll != Vec2::ZERO {
                    egui_input.events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::Vec2 {
                            x: current_state.discrete_scroll.x,
                            y: current_state.discrete_scroll.y,
                        },
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                if current_state.continuous_scroll != Vec2::ZERO {
                    egui_input.events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::Vec2 {
                            x: current_state.continuous_scroll.x,
                            y: current_state.continuous_scroll.y,
                        },
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            next_states.insert(method_ctx.input_method, current_state);
        }
        for state in mem::replace(state.entry(ctx.handler).or_default(), next_states).into_values()
        {
            if state.click {
                egui_input.events.push(egui::Event::PointerButton {
                    pos: Pos2::ZERO,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            if state.grab {
                cmds.entity(ctx.handler).remove::<GrabbedEguiWindow>();
            }
        }
    }
}

#[derive(Default)]
struct InputState {
    click: bool,
    grab: bool,
    /// How many Lines to scroll
    discrete_scroll: Vec2,
    /// How many Pixels to scroll
    continuous_scroll: Vec2,
}

pub struct SpawnSpatialEguiWindowCommand {
    pub target_entity: Option<Entity>,
    pub position: Vec3,
    pub rotation: Quat,
    pub resolution: UVec2,
    pub unlit: bool,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Component)]
pub struct SpatialEguiWindow;

impl Command for SpawnSpatialEguiWindowCommand {
    fn apply(self, world: &mut World) {
        let mut textures = world.resource_mut::<Assets<Image>>();
        let texture = textures.add({
            let size = Extent3d {
                width: self.resolution.x,
                height: self.resolution.y,
                depth_or_array_layers: 1,
            };
            let mut output_texture = Image {
                data: vec![0; (size.width * size.height * 4) as usize],
                ..default()
            };
            output_texture.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
            output_texture.texture_descriptor.size = size;
            output_texture
        });
        let mut materials = world.remove_resource::<Assets<StandardMaterial>>().unwrap();
        let mut meshes = world.remove_resource::<Assets<Mesh>>().unwrap();
        let size = Vec3::new(
            self.height * (self.resolution.y as f32 / (self.resolution.x as f32)),
            self.height,
            0.05,
        );
        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture.clone()),
            unlit: self.unlit,
            ..Default::default()
        });
        let mesh = meshes.add(construct_window_mesh(size.xy(), size.z));
        let bundle = (
            Field::Cuboid(Cuboid::from_size(size)),
            InputHandler::new(input_surface_capture_condition),
            EguiRenderToImage{
                handle: texture,
                load_op: LoadOp::Clear(wgpu_types::Color::TRANSPARENT), 
            },
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(self.position).with_rotation(self.rotation),
            SpatialEguiWindow,
            SpatialEguiWindowPhysicalSize(size),
        );
        world.insert_resource(materials);
        world.insert_resource(meshes);
        if let Some(target) = self.target_entity {
            world.entity_mut(target).insert(bundle);
        } else {
            world.spawn(bundle);
        }
    }
}

#[derive(Clone, Copy, Component, Debug)]
pub struct SpatialEguiWindowPhysicalSize(pub Vec3);

const MAX_CLOSE_RANGE_INTERACTION_DISTANCE: f32 = 0.15;

fn input_surface_capture_condition(
    ctx: In<CaptureContext>,
    method_query: Query<(
        Has<PointerInputMethod>,
        Option<&XrControllerInputMethodData>,
        Option<&HandInputMethodData>,
        Option<&MouseInputMethodData>,
    )>,
    mut giz: Gizmos,
) -> bool {
    let Ok((is_pointer_method, xr_controller_data, xr_hand_data, mouse_data)) =
        method_query.get(ctx.input_method)
    else {
        warn!("invald input method");
        return false;
    };
    if is_pointer_method {
        return true;
    }
    let distance = ctx
        .closest_point
        .distance(ctx.input_method_location.translation);
    if distance > MAX_CLOSE_RANGE_INTERACTION_DISTANCE {
        return false;
    }
    if distance <= f32::EPSILON {
        return true;
    }

    let mat = ctx.handler_location.compute_matrix();
    giz.line(
        mat.transform_point3(ctx.closest_point),
        mat.transform_point3(ctx.input_method_location.translation),
        css::WHITE,
    );

    let mut capture = false;
    if let Some(mouse) = mouse_data {
        capture |= mouse.left_button.pressed;
        capture |= mouse.right_button.pressed;
        capture |= mouse.discrete_scroll != Vec2::ZERO;
        capture |= mouse.continuous_scroll != Vec2::ZERO;
    }
    if let Some(hand) = xr_hand_data {
        let hand = hand.get_in_relative_space(&ctx.handler_location);
        capture |= hand.index.tip.pos.distance(hand.thumb.tip.pos)
            < (0.002 + hand.index.tip.radius + hand.thumb.tip.radius);
    }
    if let Some(controller) = xr_controller_data {
        capture |= controller.trigger_pulled;
        capture |= controller.squeezed;
        capture |= controller.stick_pos.y.abs() > 0.1;
    }
    capture
}
