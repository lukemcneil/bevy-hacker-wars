use std::f32::consts::PI;

use bevy::{
    input::{
        common_conditions::input_toggle_active,
        gamepad::{GamepadConnectionEvent, GamepadEvent, GamepadSettings},
    },
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Hacker Wars".into(),
                        resolution: (1000.0, 600.0).into(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .build(),
        )
        .init_resource::<BulletsAssets>()
        .register_type::<BulletsAssets>()
        .register_type::<Player>()
        .register_type::<Ball>()
        .register_type::<Velocity>()
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        )
        .add_systems(Startup, (setup, setup_gamepads, setup_bullet_assets))
        .add_systems(
            Update,
            (
                gamepad_connections,
                player_movement,
                player_rotation,
                create_bullets,
                apply_velocity,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    let camera = Camera2dBundle::default();
    // camera.projection.scaling_mode = ScalingMode::AutoMin {
    //     min_width: 256.0,
    //     min_height: 144.0,
    // };

    commands.spawn(camera);
}

fn setup_gamepads(mut settings: ResMut<GamepadSettings>) {
    let dz = 0.1;
    settings.default_axis_settings.set_deadzone_lowerbound(-dz);
    settings.default_axis_settings.set_deadzone_upperbound(dz);
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct BulletsAssets {
    mesh_handle: Handle<Mesh>,
    material_handle: Handle<ColorMaterial>,
}

fn setup_bullet_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut bullets_assets: ResMut<BulletsAssets>,
) {
    let mesh_handle = meshes.add(shape::Circle::default().into());
    let material_handle = materials.add(ColorMaterial::from(Color::rgb(1.0, 0.5, 0.5)));
    bullets_assets.mesh_handle = mesh_handle;
    bullets_assets.material_handle = material_handle;
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Player {
    speed: f32,
    rotation_speed: f32,
    gamepad_id: usize,
}

#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component)]
struct Velocity(Vec2);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Ball;

fn gamepad_connections(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut connection_events: EventReader<GamepadConnectionEvent>,
    players: Query<(Entity, &Player)>,
    windows: Query<&Window>,
) {
    for connection_event in connection_events.iter() {
        let gamepad = connection_event.gamepad;
        match &connection_event.connection {
            bevy::input::gamepad::GamepadConnection::Connected(info) => {
                let texture = asset_server.load("pikachu.png");
                let window = windows.single();
                let w = window.width() / 2.0;
                let h = window.height() / 2.0;
                commands.spawn((
                    SpriteBundle {
                        texture,
                        transform: Transform::from_translation(Vec3 {
                            x: rand::thread_rng().gen_range(-w..w),
                            y: rand::thread_rng().gen_range(-h..h),
                            z: 0.0,
                        }),
                        ..default()
                    },
                    Player {
                        speed: 500.0,
                        rotation_speed: 7000.0,
                        gamepad_id: gamepad.id,
                    },
                    Name::new(format!("Player: {}", info.name)),
                ));
            }
            bevy::input::gamepad::GamepadConnection::Disconnected => {
                for (player_entity, player) in players.iter() {
                    if player.gamepad_id == gamepad.id {
                        commands.entity(player_entity).despawn();
                        return;
                    }
                }
            }
        }
    }
}

fn player_movement(
    mut players: Query<(&mut Transform, &Player)>,
    axes: Res<Axis<GamepadAxis>>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
) {
    for gamepad in gamepads.iter() {
        for (mut transform, player) in &mut players {
            if player.gamepad_id != gamepad.id {
                continue;
            }
            let axis_lx = GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickX,
            };
            let axis_ly = GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickY,
            };
            if let (Some(x), Some(y)) = (axes.get(axis_lx), axes.get(axis_ly)) {
                let movement_amount = player.speed * time.delta_seconds();
                let mut v = Vec2 { x, y };
                if v.distance(Vec2::ZERO) > 1.0 {
                    v = v.normalize();
                }
                transform.translation.x += movement_amount * v.x;
                transform.translation.y += movement_amount * v.y;
            }
        }
    }
}

fn player_rotation(
    mut players: Query<(&mut Transform, &Player)>,
    axes: Res<Axis<GamepadAxis>>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
) {
    for gamepad in gamepads.iter() {
        for (mut transform, player) in &mut players {
            if player.gamepad_id != gamepad.id {
                continue;
            }
            let axis_rx = GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::RightStickX,
            };
            let axis_ry = GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::RightStickY,
            };
            if let (Some(x), Some(y)) = (axes.get(axis_rx), axes.get(axis_ry)) {
                let rotation_amount = player.rotation_speed * time.delta_seconds();
                let v = Vec2 { x, y };
                if v != Vec2::ZERO {
                    let target_quat = Quat::from_rotation_z(-v.angle_between(Vec2::X) - PI / 2.0);
                    let angle_between = transform.rotation.angle_between(target_quat);
                    let max_angle = rotation_amount * time.delta_seconds();
                    if angle_between > max_angle {
                        let s = max_angle / (angle_between);
                        transform.rotation = transform.rotation.slerp(target_quat, s);
                    } else {
                        transform.rotation = target_quat;
                    };
                }
            }
        }
    }
}

fn create_bullets(
    mut gamepad_evr: EventReader<GamepadEvent>,
    mut commands: Commands,
    bullets_assets: Res<BulletsAssets>,
    players: Query<(&Transform, &Player)>,
) {
    for ev in gamepad_evr.iter() {
        match ev {
            GamepadEvent::Button(button_ev) => match button_ev.button_type {
                GamepadButtonType::RightTrigger => {
                    for (transform, player) in &players {
                        if player.gamepad_id != button_ev.gamepad.id {
                            continue;
                        }
                        if button_ev.value == 1.0 {
                            let (v, mut angle) = transform.rotation.to_axis_angle();
                            angle *= v.z;
                            angle += PI / 2.0;
                            commands.spawn((
                                MaterialMesh2dBundle {
                                    mesh: bullets_assets.mesh_handle.clone().into(),
                                    material: bullets_assets.material_handle.clone(),
                                    transform: Transform::from_translation(transform.translation)
                                        .with_scale(Vec3::new(10.0, 10.0, 10.0)),
                                    ..default()
                                },
                                Ball,
                                Velocity(Vec2::from_angle(angle).rotate(Vec2::X * 600.0)),
                                Name::new("Ball"),
                            ));
                        }
                    }
                }
                _ => return,
            },
            _ => return,
        }
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_seconds();
        transform.translation.y += velocity.y * time.delta_seconds();
    }
}
