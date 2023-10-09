use std::f32::consts::PI;

use bevy::{
    input::{
        common_conditions::input_toggle_active,
        gamepad::{GamepadConnectionEvent, GamepadEvent, GamepadSettings},
    },
    prelude::*,
    sprite::{collide_aabb::collide, MaterialMesh2dBundle},
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
        .init_resource::<BulletMesh>()
        .register_type::<BulletMesh>()
        .init_resource::<PlayerMesh>()
        .register_type::<PlayerMesh>()
        .register_type::<Player>()
        .register_type::<Bullet>()
        .register_type::<Velocity>()
        .register_type::<ID>()
        .register_type::<Health>()
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        )
        .add_systems(Startup, (setup, setup_gamepads, setup_assets))
        .add_systems(
            Update,
            (
                gamepad_connections,
                player_movement,
                player_rotation,
                create_bullets,
                apply_velocity,
                despawn_bullets,
                check_for_collisions,
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
struct BulletMesh {
    mesh_handle: Handle<Mesh>,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct PlayerMesh {
    mesh_handle: Handle<Mesh>,
}

fn setup_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut bullet_mesh: ResMut<BulletMesh>,
    mut player_mesh: ResMut<PlayerMesh>,
) {
    let bullet_mesh_handle = meshes.add(shape::Circle::default().into());
    bullet_mesh.mesh_handle = bullet_mesh_handle;
    let player_mesh_handle = meshes.add(shape::Box::new(1.0, 1.0, 1.0).into());
    player_mesh.mesh_handle = player_mesh_handle;
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Player {
    speed: f32,
    rotation_speed: f32,
    material_handle: Handle<ColorMaterial>,
}

#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component)]
struct Velocity(Vec2);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Bullet;

#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component)]
struct ID(usize);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Health {
    starting_health: i32,
    current_health: i32,
}

fn gamepad_connections(
    mut commands: Commands,
    mut connection_events: EventReader<GamepadConnectionEvent>,
    players: Query<(Entity, &ID), With<Player>>,
    windows: Query<&Window>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    player_mesh: Res<PlayerMesh>,
) {
    for connection_event in connection_events.iter() {
        let gamepad = connection_event.gamepad;
        match &connection_event.connection {
            bevy::input::gamepad::GamepadConnection::Connected(info) => {
                let window = windows.single();
                let w = window.width() / 2.0;
                let h = window.height() / 2.0;
                let mut rng = rand::thread_rng();
                let material_handle = materials.add(ColorMaterial::from(Color::rgb(
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                )));
                commands.spawn((
                    MaterialMesh2dBundle {
                        mesh: player_mesh.mesh_handle.clone().into(),
                        material: material_handle.clone(),
                        transform: Transform::from_translation(Vec3 {
                            x: rand::thread_rng().gen_range(-w..w),
                            y: rand::thread_rng().gen_range(-h..h),
                            z: gamepad.id as f32,
                        })
                        .with_scale(Vec3::new(50.0, 50.0, 0.0)),
                        ..default()
                    },
                    Player {
                        speed: 500.0,
                        rotation_speed: 7000.0,
                        material_handle,
                    },
                    ID(gamepad.id),
                    Health {
                        starting_health: 5,
                        current_health: 5,
                    },
                    Name::new(format!("Player: {}", info.name)),
                ));
            }
            bevy::input::gamepad::GamepadConnection::Disconnected => {
                for (player_entity, id) in players.iter() {
                    if id.0 == gamepad.id {
                        commands.entity(player_entity).despawn();
                        return;
                    }
                }
            }
        }
    }
}

fn player_movement(
    mut players: Query<(&mut Transform, &ID, &Player)>,
    axes: Res<Axis<GamepadAxis>>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
    windows: Query<&Window>,
) {
    for gamepad in gamepads.iter() {
        for (mut transform, id, player) in &mut players {
            if id.0 != gamepad.id {
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
                let window = windows.single();
                let bounds = Vec3 {
                    x: window.width() / 2.0,
                    y: window.height() / 2.0,
                    z: f32::MAX,
                };
                transform.translation = transform.translation.clamp(-bounds, bounds);
            }
        }
    }
}

fn player_rotation(
    mut players: Query<(&mut Transform, &ID, &Player)>,
    axes: Res<Axis<GamepadAxis>>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
) {
    for gamepad in gamepads.iter() {
        for (mut transform, id, player) in &mut players {
            if id.0 != gamepad.id {
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
    bullet_mesh: Res<BulletMesh>,
    players: Query<(&Transform, &ID, &Player)>,
) {
    for ev in gamepad_evr.iter() {
        match ev {
            GamepadEvent::Button(button_ev) => match button_ev.button_type {
                GamepadButtonType::RightTrigger => {
                    for (transform, id, player) in &players {
                        if id.0 != button_ev.gamepad.id {
                            continue;
                        }
                        if button_ev.value == 1.0 {
                            let (v, mut angle) = transform.rotation.to_axis_angle();
                            angle *= v.z;
                            angle += PI / 2.0;
                            commands.spawn((
                                MaterialMesh2dBundle {
                                    mesh: bullet_mesh.mesh_handle.clone().into(),
                                    material: player.material_handle.clone(),
                                    transform: Transform::from_translation(transform.translation)
                                        .with_scale(Vec3::new(10.0, 10.0, 0.0)),
                                    ..default()
                                },
                                Bullet,
                                ID(button_ev.gamepad.id),
                                Velocity(Vec2::from_angle(angle).rotate(Vec2::X * 600.0)),
                                Name::new("Bullet"),
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

fn despawn_bullets(
    query: Query<(Entity, &Transform), With<Bullet>>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let window = windows.single();
    for (entity, transform) in &query {
        if transform.translation.x < -window.width() / 2.0
            || transform.translation.x > window.width() / 2.0
            || transform.translation.y < -window.height() / 2.0
            || transform.translation.y > window.height() / 2.0
        {
            commands.entity(entity).despawn();
        }
    }
}

fn check_for_collisions(
    bullet_query: Query<(Entity, &Transform, &ID), With<Bullet>>,
    mut player_query: Query<(Entity, &ID, &Transform, &mut Health), With<Player>>,
    mut commands: Commands,
) {
    for (player_entity, player_id, player_transform, mut player_health) in &mut player_query {
        for (bullet_entity, bullet_transform, bullet_id) in &bullet_query {
            if player_id.0 == bullet_id.0 {
                continue;
            }
            let collision = collide(
                player_transform.translation,
                player_transform.scale.truncate(),
                bullet_transform.translation,
                bullet_transform.scale.truncate(),
            );
            if let Some(_) = collision {
                commands.entity(bullet_entity).despawn();

                player_health.current_health -= 1;
                if player_health.current_health == 0 {
                    commands.entity(player_entity).despawn();
                }
            }
        }
    }
}
