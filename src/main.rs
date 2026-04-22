use bevy::prelude::*;
use avian3d::prelude::*;

#[derive(Component)]
struct Player;
#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (movment, rotate_to_mouse, camera_follow))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Podloga
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(60.0, 0.2, 60.0),
        Mesh3d(meshes.add(Cuboid::new(60.0, 0.2, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));

    // Gracz
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.4),
        LockedAxes::ROTATION_LOCKED,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ))
    .with_children(|parent| {
        parent.spawn((
            SceneRoot(asset_server.load("player.glb#Scene0")),
            Transform::from_xyz(0.0, 1.0, 0.0),
        ));
    });

    // Kamera
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 12.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn movment(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut LinearVelocity, With<Player>>,
    _time: Res<Time>,
) {
    let speed = 5.0;
    for mut velocity in &mut query {
        let mut direction = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) { direction.z -= 1.0; }
        if keys.pressed(KeyCode::KeyS) { direction.z += 1.0; }
        if keys.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if keys.pressed(KeyCode::KeyD) { direction.x += 1.0; }
        if direction.length() > 0.0 {
            direction = direction.normalize();
        }
        velocity.x = direction.x * speed;
        velocity.z = direction.z * speed;
    }
}

fn rotate_to_mouse(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,  // <- With<MainCamera>
    mut players: Query<&mut Transform, With<Player>>,
) {
    let Ok(window) = windows.single() else { return; };
    let Ok((camera, camera_transform)) = cameras.single() else { return; };
    let Ok(mut player_transform) = players.single_mut() else { return; };

    let Some(cursor_pos) = window.cursor_position() else { return; };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else { return; };
    let Some(distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) else { return; };

    let world_pos = ray.get_point(distance);
    let direction = world_pos - player_transform.translation;

    if direction.length() > 0.1 {
        player_transform.look_to(
            Vec3::new(direction.x, 0.0, direction.z).normalize(),
            Vec3::Y,
        );
    }
}

fn camera_follow(
    player: Query<&Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = player.single() else { return; };
    let Ok(mut camera_transform) = camera.single_mut() else { return; };
    let offset = Vec3::new(0.0, 12.0, 7.0);
    camera_transform.translation = player_transform.translation + offset;
}