pub mod window_mesh;

use bevy::{
    asset::RenderAssetUsages,
    ecs::entity::EntityHashSet,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    window::PrimaryWindow,
};
use bevy_egui::{
    egui::{self, TouchDeviceId, TouchId},
    EguiContext, EguiInput, EguiPreUpdateSet, EguiRenderToImage,
};
use bevy_suis::{
    field::Field,
    handler_actions::multi::MultiHandlerAction,
    input_handler::InputHandler,
    input_method_data::{InputData, SpatialInputData},
};
use window_mesh::construct_window_mesh;

pub struct SpatialEguiPlugin;

impl Plugin for SpatialEguiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_spatial_window_input);
        app.add_systems(
            PreUpdate,
            forward_egui_events
                .after(EguiPreUpdateSet::ProcessInput)
                .before(EguiPreUpdateSet::BeginPass),
        );
    }
}

fn forward_egui_events(
    mut query: Query<&mut EguiInput, With<SpatialEguiWindow>>,
    window_query: Query<&EguiInput, (With<PrimaryWindow>, Without<SpatialEguiWindow>)>,
) {
    let Ok(primary_input) = window_query.single() else {
        warn_once!("Unable to find one Primary Window!");
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

fn update_spatial_window_input(
    mut windows: Query<(
        &SpatialEguiWindow,
        &mut InputHandler,
        &mut MultiHandlerAction,
        &mut EguiInput,
        &mut EguiContext,
        &mut SpatialEguiState,
        &Field,
        &GlobalTransform,
    )>,
    mut gizmos: Gizmos,
) {
    for (
        window,
        mut handler,
        mut action,
        mut egui_input,
        mut egui_context,
        mut state,
        field,
        transform,
    ) in &mut windows
    {
        action.update(
            &mut handler,
            |data| match data.spatial_data {
                SpatialInputData::Hand(_) | SpatialInputData::Tip(_) => dbg!(data.distance) < 0.15,
                SpatialInputData::Ray(_) => data.distance <= 0.0,
            },
            |data| {
                let n = data.non_spatial_data;
                let input = n.select > 0.8
                    || n.secondary > 0.8
                    || n.context > 0.8
                    || n.scroll.is_some_and(|v| v != Vec2::ZERO);
                input
                    || match data.spatial_data {
                        SpatialInputData::Hand(hand) => {
                            // the input is in the same space as the field, since its on the same
                            // entity
                            field.distance(&GlobalTransform::IDENTITY, hand.index.tip.pos)
                                <= hand.index.tip.radius
                        }
                        SpatialInputData::Tip(_) => data.distance <= 0.0,
                        // shouldn't be captured based on distance
                        SpatialInputData::Ray(_) => false,
                    }
            },
        );
        if action.hover_set().current().is_empty() {
            egui_input.events.push(egui::Event::PointerGone);
        }
        if let Some(actor) = action
            .currently_hovering(&handler)
            .filter(|data| match data.spatial_data {
                SpatialInputData::Hand(_) | SpatialInputData::Tip(_) => {
                    (0.03..0.15).contains(&data.distance)
                }
                SpatialInputData::Ray(_) => data.distance < 0.01,
            })
            .reduce(|data_1, data_2| {
                if data_1.distance < data_2.distance {
                    data_1
                } else {
                    data_2
                }
            })
        {
            let interact_point = cursor_interact_point(field, &actor.spatial_data);
            // point already in field space
            let closest_point = field.closest_point(&GlobalTransform::IDENTITY, interact_point);
            {
                let p = transform.transform_point(interact_point.into());
                let closest = transform.transform_point(closest_point.into());
                gizmos.line(p, closest, Color::srgb(0.0, 1.0, 0.5));
            }
            let uv = ((closest_point.xy() / window.physical_size.xy()) * vec2(1.0, -1.0)) + 0.5;
            let pos = egui::Pos2 {
                x: (uv.x * window.resolution.x as f32) / egui_context.get_mut().pixels_per_point(),
                y: (uv.y * window.resolution.y as f32) / egui_context.get_mut().pixels_per_point(),
            };
            egui_input.events.push(egui::Event::PointerMoved(pos));
            check(
                actor.non_spatial_data.select > 0.8,
                &mut state.left_clicked,
                |pressed| {
                    egui_input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    });
                },
            );
            check(
                actor.non_spatial_data.secondary > 0.8,
                &mut state.right_clicked,
                |pressed| {
                    egui_input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    });
                },
            );
            check(
                actor.non_spatial_data.context > 0.8,
                &mut state.middle_clicked,
                |pressed| {
                    egui_input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Middle,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    });
                },
            );
            if let Some(scroll) = actor
                .non_spatial_data
                .scroll
                .filter(|scroll| scroll != &Vec2::ZERO)
            {
                egui_input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: scroll.to_array().into(),
                    modifiers: egui::Modifiers::NONE,
                });
            }
        }
        for actor in action
            .started_acting(&handler)
            .filter(|i| touch_interact_distance(i, field) <= 0.0)
        {
            state.touching.insert(actor.input_method);
            // this is already relative to the Field
            let touch_point = touch_interact_point(&actor);

            let uv = ((touch_point.xy() / window.physical_size.xy()) * vec2(1.0, -1.0)) + 0.5;
            let pos = egui::Pos2 {
                x: (uv.x * window.resolution.x as f32) / egui_context.get_mut().pixels_per_point(),
                y: (uv.y * window.resolution.y as f32) / egui_context.get_mut().pixels_per_point(),
            };
            egui_input.events.push(egui::Event::Touch {
                device_id: TouchDeviceId(0),
                id: TouchId(actor.input_method.to_bits()),
                phase: egui::TouchPhase::Start,
                pos: pos,
                force: None,
            });
            egui_input.events.push(egui::Event::PointerButton {
                pos: pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
        }
        for actor in action
            .currently_acting(&handler)
            .filter(|i| state.touching.contains(&i.input_method))
        {
            let touch_point = touch_interact_point(&actor);
            // let uv = closest_point.xy() / window.physical_size.xy();

            let uv = ((touch_point.xy() / window.physical_size.xy()) * vec2(1.0, -1.0)) + 0.5;
            let pos = egui::Pos2 {
                x: (uv.x * window.resolution.x as f32) / egui_context.get_mut().pixels_per_point(),
                y: (uv.y * window.resolution.y as f32) / egui_context.get_mut().pixels_per_point(),
            };
            egui_input.events.push(egui::Event::Touch {
                device_id: TouchDeviceId(0),
                id: TouchId(actor.input_method.to_bits()),
                phase: egui::TouchPhase::Move,
                pos: pos,
                force: None,
            });
            egui_input.events.push(egui::Event::PointerMoved(pos));
        }
        for actor in action.stopped_acting(&handler) {
            if !state.touching.remove(&actor.input_method) {
                continue;
            }
            // this is already relative to the Field
            let touch_point = touch_interact_point(&actor);

            let uv = ((touch_point.xy() / window.physical_size.xy()) * vec2(1.0, -1.0)) + 0.5;
            let pos = egui::Pos2 {
                x: (uv.x * window.resolution.x as f32) / egui_context.get_mut().pixels_per_point(),
                y: (uv.y * window.resolution.y as f32) / egui_context.get_mut().pixels_per_point(),
            };
            egui_input.events.push(egui::Event::Touch {
                device_id: TouchDeviceId(0),
                id: TouchId(actor.input_method.to_bits()),
                phase: egui::TouchPhase::End,
                pos: pos,
                force: None,
            });
            egui_input.events.push(egui::Event::PointerButton {
                pos: pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
    }
}

fn touch_interact_distance(data: &InputData, field: &Field) -> f32 {
    match data.spatial_data {
        SpatialInputData::Hand(hand) => {
            // the input is in the same space as the field, since its on the same
            // entity
            field.distance(&GlobalTransform::IDENTITY, hand.index.tip.pos) - hand.index.tip.radius
        }
        SpatialInputData::Tip(_) => data.distance,
        // shouldn't be captured based on distance
        SpatialInputData::Ray(_) => f32::MAX,
    }
}
fn touch_interact_point(data: &InputData) -> Vec3 {
    match data.spatial_data {
        SpatialInputData::Hand(hand) => hand.index.tip.pos,
        SpatialInputData::Tip(tip) => tip.translation.into(),
        // shouldn't be captured based on distance
        SpatialInputData::Ray(_) => Vec3::ZERO,
    }
}

fn cursor_interact_point(field: &Field, input: &SpatialInputData) -> Vec3A {
    match input {
        SpatialInputData::Hand(hand) => hand.index.tip.pos.lerp(hand.thumb.tip.pos, 0.5).into(),
        SpatialInputData::Tip(tip) => tip.translation,
        SpatialInputData::Ray(ray) => ray
            .get_point(
                field
                    .raymarch(&GlobalTransform::IDENTITY, *ray)
                    .deepest_point_ray_length,
            )
            .into(),
    }
}

fn check(curr: bool, state: &mut bool, mut func: impl FnMut(bool)) {
    if curr && !*state {
        func(true)
    }
    if !curr && *state {
        func(false)
    }
    *state = curr;
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
#[require(SpatialEguiState)]
pub struct SpatialEguiWindow {
    pub physical_size: Vec3,
    pub resolution: UVec2,
}
#[derive(Default, Component)]
struct SpatialEguiState {
    touching: EntityHashSet,
    left_clicked: bool,
    right_clicked: bool,
    middle_clicked: bool,
}

impl Command for SpawnSpatialEguiWindowCommand {
    fn apply(self, world: &mut World) {
        let mut textures = world.resource_mut::<Assets<Image>>();
        let texture = textures.add({
            let size = Extent3d {
                width: self.resolution.x,
                height: self.resolution.y,
                depth_or_array_layers: 1,
            };
            let mut output_texture = Image::new_uninit(
                size,
                TextureDimension::D2,
                TextureFormat::bevy_default(),
                // main world needed for reading resolution
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            output_texture.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
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
            MultiHandlerAction::default(),
            Field::Cuboid(Cuboid::from_size(size)),
            InputHandler::new(bevy_suis::input_handler::FieldRef::This),
            EguiRenderToImage::new(texture),
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(self.position).with_rotation(self.rotation),
            SpatialEguiWindow {
                physical_size: size,
                resolution: self.resolution,
            },
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
