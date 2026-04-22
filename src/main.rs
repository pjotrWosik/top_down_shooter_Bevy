use bevy::prelude::*;
use avian3d::prelude::*;


#[derive(Component)]
struct Player;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Bullet {
    lifetime: f32,
}

#[derive(Component)]
struct Enemy {
    health: f32,
    speed: f32,
}


fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (movment, rotate_to_mouse, camera_follow, shoot, bullet_lifetime))
        .add_systems(Update, (enemy_movment, bullet_hit_enemy))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Swiatlo
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Podloga
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(60.0, 0.2, 60.0),
        Mesh3d(meshes.add(Cuboid::new(60.0, 0.2, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));   
    
    // Kamera
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 12.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
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

    // Wrug
    commands.spawn((
        Enemy { health: 20.0, speed: 2.0 },
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.4),
        LockedAxes::ROTATION_LOCKED,
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.0, 0.0))),
        Transform::from_xyz(5.0, 1.5, 5.0),
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

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    player: Query<&Transform, With<Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player.single() else { return; };

    if mouse.just_pressed(MouseButton::Left) {
        let direction = player_transform.forward().as_vec3();

        commands.spawn((
            Bullet { lifetime: 3.0 },
            RigidBody::Dynamic,
            Collider::sphere(0.1),
            GravityScale(0.0),
            LinearVelocity(direction * 20.0),
            Mesh3d(meshes.add(Sphere::new(0.1))),
            MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 0.0))),
            Transform::from_translation(
                player_transform.translation + direction * 1.0 + Vec3::Y * 0.5
            ),
        ));
    }
}

fn bullet_lifetime(
    mut commands: Commands,
    mut bullets: Query<(Entity, &mut Bullet)>,
    time: Res<Time>,
) {
    for (entity, mut bullet) in &mut bullets {
        bullet.lifetime -= time.delta_secs();
        if bullet.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_movment(
    mut enemies: Query<(&mut LinearVelocity, &Transform, &Enemy)>,
    player: Query<&Transform, With<Player>>,
) {
    let Ok(player_transform) = player.single() else { return; };

    for (mut velocity, enemy_transform, enemy) in &mut enemies {
        let direction = player_transform.translation - enemy_transform.translation;
        let direction = Vec3::new(direction.x, 0.0, direction.z);

        if direction.length() > 0.5 {
            let dir = direction.normalize();
            velocity.x = dir.x * enemy.speed;
            velocity.z = dir.z * enemy.speed;
        } else {
            velocity.x = 0.0;
            velocity.z = 0.0;
        }
    }
}

fn bullet_hit_enemy(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy)>,
) {
    for (bullet_entity, bullet_transform) in &bullets {
        for (enemy_entity, enemy_transform, mut enemy) in &mut enemies {
            let distance = bullet_transform.translation
                .distance(enemy_transform.translation);

            if distance < 1.0 {
                enemy.health -= 8.0;  // 8hp per strzał
                commands.entity(bullet_entity).despawn();

                if enemy.health <= 0.0 {
                    commands.entity(enemy_entity).despawn();
                }
            }
        }
    }
}
