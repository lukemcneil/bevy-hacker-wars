use std::{f32::consts::PI, time::Duration};

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::{
        common_conditions::input_toggle_active,
        gamepad::{GamepadConnectionEvent, GamepadEvent, GamepadSettings},
    },
    prelude::*,
    sprite::{collide_aabb::collide, MaterialMesh2dBundle},
    utils::HashSet,
};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::{bevy_egui::EguiContexts, egui::Slider};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Hacker Wars".into(),
                        resolution: (1500.0, 1000.0).into(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .build(),
        )
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            // WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
            EguiPlugin,
        ))
        .register_type::<PlayerConfig>()
        .register_type::<BulletConfig>()
        .init_resource::<BulletMesh>()
        .register_type::<BulletMesh>()
        .init_resource::<PlayerMesh>()
        .register_type::<PlayerMesh>()
        .register_type::<Player>()
        .register_type::<Bullet>()
        .register_type::<Collider>()
        .register_type::<Alive>()
        .register_type::<Velocity>()
        .register_type::<ID>()
        .register_type::<Health>()
        .register_type::<Shooter>()
        .add_event::<PlayerConfigChanged>()
        .add_event::<PlayerDied>()
        .insert_resource(PlayerConfig {
            speed: 500.0,
            turning_speed: 13.0,
            shooting_delay: 0.1,
            scale: 50.0,
            invincible: false,
            starting_health: 10,
        })
        .insert_resource(BulletConfig {
            speed: 600.0,
            collide: true,
            scale: 10.0,
        })
        .add_systems(Startup, (setup_camera, setup_gamepads, setup_assets))
        .add_systems(
            Update,
            (
                gamepad_connections,
                player_movement,
                player_rotation,
                create_bullets,
                apply_velocity,
                despawn_bullets,
                check_for_collisions
                    .after(apply_velocity)
                    .after(player_movement),
                // bounce_bullets,
                config_ui_system.run_if(input_toggle_active(true, KeyCode::Escape)),
                respond_to_player_config_change,
                handle_buttons,
                kill_player.after(player_movement),
            ),
        )
        .run();
}

#[derive(Event, Default)]
struct PlayerConfigChanged;

#[derive(Event, Default)]
struct PlayerDied {
    id: usize,
}

fn config_ui_system(
    mut contexts: EguiContexts,
    mut player_config: ResMut<PlayerConfig>,
    mut bullet_config: ResMut<BulletConfig>,
    mut ev_player_config_changed: EventWriter<PlayerConfigChanged>,
) {
    bevy_inspector_egui::egui::Window::new("Settings").show(contexts.ctx_mut(), |ui| {
        ui.add(Slider::new(&mut player_config.speed, 50.0..=1000.0).text("player speed"));
        ui.add(Slider::new(&mut player_config.turning_speed, 1.0..=50.0).text("turning speed"));
        if ui
            .add(Slider::new(&mut player_config.shooting_delay, 0.0..=1.0).text("shooting delay"))
            .changed()
            || ui
                .add(Slider::new(&mut player_config.scale, 10.0..=100.0).text("player size"))
                .changed()
            || ui
                .checkbox(&mut player_config.invincible, "invincible")
                .changed()
            || ui
                .add(Slider::new(&mut player_config.starting_health, 1..=1000).text("health"))
                .changed()
        {
            ev_player_config_changed.send_default();
        };

        ui.add(Slider::new(&mut bullet_config.speed, 50.0..=1500.0).text("bullet speed"));
        ui.checkbox(&mut bullet_config.collide, "bullets collide");
        ui.add(Slider::new(&mut bullet_config.scale, 1.0..=100.0).text("bullet size"));
    });
}

fn respond_to_player_config_change(
    mut ev_player_config_changed: EventReader<PlayerConfigChanged>,
    mut shooters: Query<(Entity, &mut Shooter, &mut Transform, &mut Health), With<Player>>,
    player_config: Res<PlayerConfig>,
    mut commands: Commands,
) {
    for _ in ev_player_config_changed.iter() {
        for (shooter_entity, mut shooter, mut transform, mut health) in &mut shooters {
            shooter
                .timer
                .set_duration(Duration::from_secs_f32(player_config.shooting_delay));
            transform.scale = Vec3 {
                x: player_config.scale,
                y: player_config.scale,
                z: 0.0,
            };
            if player_config.invincible {
                commands.entity(shooter_entity).remove::<Collider>();
            } else {
                commands.entity(shooter_entity).insert(Collider);
            }
            health.current_health = player_config.starting_health;
        }
    }
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

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct PlayerConfig {
    speed: f32,
    turning_speed: f32,
    shooting_delay: f32,
    scale: f32,
    invincible: bool,
    starting_health: i32,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct BulletConfig {
    speed: f32,
    collide: bool,
    scale: f32,
}

fn setup_camera(mut commands: Commands) {
    let camera = Camera2dBundle::default();
    commands.spawn(camera);
}

fn setup_gamepads(mut settings: ResMut<GamepadSettings>) {
    let dz = 0.1;
    settings.default_axis_settings.set_deadzone_lowerbound(-dz);
    settings.default_axis_settings.set_deadzone_upperbound(dz);
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
    material_handle: Handle<ColorMaterial>,
}

#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component)]
struct Velocity(Vec2);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Shooter {
    timer: Timer,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Bullet;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Collider;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Alive;

#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component)]
struct ID(usize);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Health {
    current_health: i32,
}

fn gamepad_connections(
    mut commands: Commands,
    mut connection_events: EventReader<GamepadConnectionEvent>,
    players: Query<(Entity, &ID), With<Player>>,
    windows: Query<&Window>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    player_mesh: Res<PlayerMesh>,
    player_config: Res<PlayerConfig>,
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
                        .with_scale(Vec3::new(player_config.scale, player_config.scale, 0.0))
                        .with_rotation(Quat::from_rotation_z(
                            rand::thread_rng().gen_range(0.0..2.0 * PI),
                        )),
                        ..default()
                    },
                    Player { material_handle },
                    Collider,
                    ID(gamepad.id),
                    Health {
                        current_health: player_config.starting_health,
                    },
                    Shooter {
                        timer: Timer::from_seconds(
                            player_config.shooting_delay,
                            TimerMode::Repeating,
                        ),
                    },
                    Alive,
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
    mut players: Query<(&mut Transform, &ID), (With<Player>, With<Alive>)>,
    axes: Res<Axis<GamepadAxis>>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
    windows: Query<&Window>,
    player_config: Res<PlayerConfig>,
) {
    for gamepad in gamepads.iter() {
        for (mut transform, id) in &mut players {
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
                let movement_amount = player_config.speed * time.delta_seconds();
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
    mut players: Query<(&mut Transform, &ID), (With<Player>, With<Alive>)>,
    axes: Res<Axis<GamepadAxis>>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
    player_config: Res<PlayerConfig>,
) {
    for gamepad in gamepads.iter() {
        for (mut transform, id) in &mut players {
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
                let v = Vec2 { x, y };
                if v != Vec2::ZERO {
                    let target_quat = Quat::from_rotation_z(-v.angle_between(Vec2::X) - PI / 2.0);
                    let angle_between = transform.rotation.angle_between(target_quat);
                    let max_angle = player_config.turning_speed * time.delta_seconds();
                    if angle_between > max_angle {
                        let s = max_angle / angle_between;
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
    mut commands: Commands,
    bullet_mesh: Res<BulletMesh>,
    mut players: Query<(&Transform, &ID, &Player, &mut Shooter), With<Alive>>,
    time: Res<Time>,
    bullet_config: Res<BulletConfig>,
) {
    for (transform, id, player, mut shooter) in &mut players {
        shooter.timer.tick(time.delta());

        if shooter.timer.just_finished() {
            let (v, mut angle) = transform.rotation.to_axis_angle();
            angle *= v.z;
            angle += PI / 2.0;
            let mut bullet_commands = commands.spawn((
                MaterialMesh2dBundle {
                    mesh: bullet_mesh.mesh_handle.clone().into(),
                    material: player.material_handle.clone(),
                    transform: Transform::from_translation(transform.translation)
                        .with_scale(Vec3::new(bullet_config.scale, bullet_config.scale, 0.0)),
                    ..default()
                },
                Bullet,
                ID(id.0),
                Velocity(Vec2::from_angle(angle).rotate(Vec2::X) * bullet_config.speed),
                Name::new("Bullet"),
            ));
            if bullet_config.collide {
                bullet_commands.insert(Collider);
            }
        }
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_seconds();
        transform.translation.y += velocity.y * time.delta_seconds();
    }
}

// fn bounce_bullets(
//     mut query: Query<(&Transform, &mut Velocity), With<Bullet>>,
//     windows: Query<&Window>,
// ) {
//     let window = windows.single();
//     for (transform, mut velocity) in &mut query {
//         if transform.translation.x < -window.width() / 2.0
//             || transform.translation.x > window.width() / 2.0
//         {
//             velocity.x = -velocity.x;
//         }
//         if transform.translation.y < -window.height() / 2.0
//             || transform.translation.y > window.height() / 2.0
//         {
//             velocity.y = -velocity.y;
//         }
//     }
// }

fn despawn_bullets(
    mut query: Query<(Entity, &Transform), With<Bullet>>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let window = windows.single();
    for (entity, transform) in &mut query {
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
    bullet_query: Query<(Entity, &ID, &Transform), With<Bullet>>,
    mut hit_query: Query<(Entity, &ID, &Transform, Option<&mut Health>), With<Collider>>,
    mut commands: Commands,
    mut ev_player_died: EventWriter<PlayerDied>,
) {
    let mut bullets_despawned = HashSet::new();
    for (bullet_entity, bullet_id, bullet_transform) in &bullet_query {
        if bullets_despawned.contains(&bullet_entity) {
            continue;
        }
        for (hit_entity, hit_id, hit_transform, player_health) in &mut hit_query {
            if bullets_despawned.contains(&hit_entity) {
                continue;
            }
            if hit_id.0 == bullet_id.0 {
                continue;
            }
            let collision = collide(
                hit_transform.translation,
                hit_transform.scale.truncate(),
                bullet_transform.translation,
                bullet_transform.scale.truncate(),
            );
            if let Some(_) = collision {
                commands.entity(bullet_entity).despawn();
                bullets_despawned.insert(bullet_entity);
                match player_health {
                    Some(mut player_health) => {
                        player_health.current_health -= 1;
                        if player_health.current_health == 0 {
                            ev_player_died.send(PlayerDied { id: hit_id.0 });
                        }
                    }
                    None => {
                        commands.entity(hit_entity).despawn();
                        bullets_despawned.insert(hit_entity);
                    }
                }
                break;
            }
        }
    }
}

fn kill_player(
    mut ev_player_died: EventReader<PlayerDied>,
    mut players: Query<(Entity, &ID, &mut Transform), (With<Player>, With<Alive>)>,
    mut commands: Commands,
) {
    for ev in ev_player_died.iter() {
        for (entity, id, mut transform) in &mut players {
            if id.0 == ev.id {
                commands.entity(entity).remove::<Alive>();
                transform.translation.x = f32::MAX;
                transform.translation.y = f32::MAX;
            }
        }
    }
}

fn handle_buttons(
    mut gamepad_evr: EventReader<GamepadEvent>,
    mut players: Query<(Entity, &ID, Option<&Alive>, &mut Transform, &mut Health), With<Player>>,
    mut commands: Commands,
    windows: Query<&Window>,
    player_config: Res<PlayerConfig>,
    mut ev_player_died: EventWriter<PlayerDied>,
) {
    for ev in gamepad_evr.iter() {
        match ev {
            GamepadEvent::Button(button_ev) => match button_ev.button_type {
                GamepadButtonType::Mode => {
                    if button_ev.value == 1.0 {
                        for (entity, id, alive_option, mut transform, mut health) in &mut players {
                            if id.0 == button_ev.gamepad.id {
                                match alive_option {
                                    Some(_) => {
                                        ev_player_died.send(PlayerDied { id: id.0 });
                                    }
                                    None => {
                                        commands.entity(entity).insert(Alive);
                                        let window = windows.single();
                                        let w = window.width() / 2.0;
                                        let h = window.height() / 2.0;
                                        transform.translation.x =
                                            rand::thread_rng().gen_range(-w..w);
                                        transform.translation.y =
                                            rand::thread_rng().gen_range(-h..h);
                                        health.current_health = player_config.starting_health;
                                    }
                                }
                                return;
                            }
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }
}
