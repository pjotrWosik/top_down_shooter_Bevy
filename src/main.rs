use bevy::prelude::*;
use avian3d::prelude::*;
use bevy::state::app::AppExtStates;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Menu,
    Playing,
}

#[derive(Component)]
struct AnimationSetupDone;

#[derive(Component)]
struct Player;

#[derive(Resource)]
struct AnimationLibrary {
    idle_gun: AnimationNodeIndex,
    run: AnimationNodeIndex,
    run_back: AnimationNodeIndex,
    run_left: AnimationNodeIndex,
    run_right: AnimationNodeIndex,

    graph: Handle<AnimationGraph>,
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Bullet {
    lifetime: f32,
}

#[derive(Resource)]
struct AmmoState {
    current: u32,
    max: u32,
    reloading: bool,
    reload_timer: f32,
    fire_timer: f32,
}

#[derive(Component)]
struct Enemy {
    health: f32,
    speed: f32,
}

#[derive(Component)]
struct Health {
    hp: f32,
}

impl Default for AmmoState {
    fn default() -> Self {
        Self { current: 30, max: 30, reloading: false, reload_timer: 0.0, fire_timer: 0.0 }
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .init_state::<GameState>()
        .add_systems(OnEnter(GameState::Menu), spawn_menu) 
        .add_systems(OnEnter(GameState::Playing), (setup, reset_ammo))
        .add_systems(Update, menu_ui.run_if(in_state(GameState::Menu)))
        .add_systems(Update, (movment, rotate_to_mouse, camera_follow, shoot, bullet_lifetime).run_if(in_state(GameState::Playing)))
        .add_systems(Update, reload.run_if(in_state(GameState::Playing)))
        .add_systems(Update, (enemy_movment, bullet_hit_enemy, enemy_damage_player, spawn_on_death).run_if(in_state(GameState::Playing)))
        .add_systems(Update, (setup_animation, animate_player).run_if(in_state(GameState::Playing)))
        .insert_resource(AmmoState::default())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        DespawnOnExit(GameState::Playing),
    ));

    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(60.0, 0.2, 60.0),
        Mesh3d(meshes.add(Cuboid::new(60.0, 0.2, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -0.1, 0.0),
        DespawnOnExit(GameState::Playing),
    ));

    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 12.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
        DespawnOnExit(GameState::Playing),
    ));

    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.4),
        LockedAxes::ROTATION_LOCKED,
        Health { hp: 20.0 },
        Transform::from_xyz(0.0, 0.0, 0.0),
        DespawnOnExit(GameState::Playing),
    ))
    .with_children(|parent| {
        parent.spawn((
            SceneRoot(asset_server.load("SWAT.glb#Scene0")),
            Transform::from_xyz(0.0, -1.0, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ))
        .with_children(|parent| {
            parent.spawn((
                SceneRoot(asset_server.load("Assault Rifle.glb#Scene0")),
                Transform::from_xyz(0.1, 1.4, 0.7)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                    .with_scale(Vec3::splat(0.2)),
            ));
        });
    });

    commands.spawn((
        Enemy { health: 20.0, speed: 2.0 },
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.4),
        LockedAxes::ROTATION_LOCKED,
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.0, 0.0))),
        Transform::from_xyz(5.0, 1.5, 5.0),
        DespawnOnExit(GameState::Playing),
    ));

    let idle_handle: Handle<AnimationClip> = asset_server.load("SWAT.glb#Animation5");
    let run_handle: Handle<AnimationClip> = asset_server.load("SWAT.glb#Animation14");
    let run_back_handle: Handle<AnimationClip> = asset_server.load("SWAT.glb#Animation15");
    let run_left_handle: Handle<AnimationClip> = asset_server.load("SWAT.glb#Animation16");
    let run_right_handle: Handle<AnimationClip> = asset_server.load("SWAT.glb#Animation17");

    // Build a single AnimationGraph with all clips (recommended)
    let mut graph = AnimationGraph::new();

    let idle_index = graph.add_clip(idle_handle.clone(), 1.0, graph.root);
    let run_index = graph.add_clip(run_handle.clone(), 1.0, graph.root);
    let run_back_index = graph.add_clip(run_back_handle.clone(), 1.0, graph.root);
    let run_left_index = graph.add_clip(run_left_handle.clone(), 1.0, graph.root);
    let run_right_index = graph.add_clip(run_right_handle.clone(), 1.0, graph.root);
    let graph_handle = animation_graphs.add(graph);

    commands.insert_resource(AnimationLibrary {
        idle_gun: idle_index,
        run: run_index,
        run_back: run_back_index,
        run_left: run_left_index,
        run_right: run_right_index,
        graph: graph_handle.clone(),
    });
}

fn menu_ui(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Playing);
        }
    }
}

fn reset_ammo(mut ammo: ResMut<AmmoState>) {
    *ammo = AmmoState::default();
}

fn spawn_menu(mut commands: Commands) {
    // Kamera potrzebna do renderowania UI
    commands.spawn((
        Camera2d,
        DespawnOnExit(GameState::Menu),
    ));

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        DespawnOnExit(GameState::Menu),
    )).with_children(|parent| {
        parent.spawn((
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(65.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("START"),
                TextFont { font_size: 40.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
    });
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
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
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
    camera_transform.translation = player_transform.translation + Vec3::new(0.0, 12.0, 7.0);
}

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    player: Query<&Transform, With<Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ammo: ResMut<AmmoState>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player.single() else { return; };
    ammo.fire_timer -= time.delta_secs();
    if mouse.pressed(MouseButton::Left) {
        if ammo.reloading || ammo.current == 0 { return; }
        if ammo.fire_timer > 0.0 { return; }
        ammo.fire_timer = 60.0 / 600.0;
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
        ammo.current -= 1;
        if ammo.current == 0 {
            ammo.reloading = true;
            ammo.reload_timer = 1.5;
        }
    }
}

fn reload(keys: Res<ButtonInput<KeyCode>>, mut ammo: ResMut<AmmoState>, time: Res<Time>) {
    if keys.just_pressed(KeyCode::KeyR) && !ammo.reloading && ammo.current < ammo.max {
        ammo.reloading = true;
        ammo.reload_timer = 1.5;
        println!("Przeładowywanie...");
    }
    if ammo.reloading {
        ammo.reload_timer -= time.delta_secs();
        if ammo.reload_timer <= 0.0 {
            ammo.current = ammo.max;
            ammo.reloading = false;
            println!("Przeładowano!");
        }
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
            let distance = bullet_transform.translation.distance(enemy_transform.translation);
            if distance < 1.0 {
                enemy.health -= 8.0;
                commands.entity(bullet_entity).despawn();
                if enemy.health <= 0.0 {
                    commands.entity(enemy_entity).despawn();
                }
            }
        }
    }
}

fn enemy_damage_player(
    enemies: Query<&Transform, With<Enemy>>,
    mut players: Query<(Entity, &Transform, &mut Health), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    time: Res<Time>,
) {
    if *current_state.get() != GameState::Playing { return; }
    let Ok((_player_entity, player_transform, mut health)) = players.single_mut() else { return; };
    for enemy_transform in &enemies {
        let distance = enemy_transform.translation.distance(player_transform.translation);
        if distance < 1.2 {
            health.hp -= 5.0 * time.delta_secs();
            if health.hp <= 0.0 {
                next_state.set(GameState::Menu);
            }
        }
    }
}

fn spawn_on_death(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _enemies: Query<Entity, With<Enemy>>,
    mut dead: RemovedComponents<Enemy>,
) {
    for _ in dead.read() {
        spawn_enemy(&mut commands, &mut meshes, &mut materials);
        if rand::random::<f32>() < 0.3 {
            spawn_enemy(&mut commands, &mut meshes, &mut materials);
        }
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    use std::f32::consts::PI;
    let angle = rand::random::<f32>() * 2.0 * PI;
    let distance = 8.0 + rand::random::<f32>() * 5.0;
    let x = angle.cos() * distance;
    let z = angle.sin() * distance;
    commands.spawn((
        Enemy { health: 24.0, speed: 2.0 },
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.4),
        LockedAxes::ROTATION_LOCKED,
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.0, 0.0))),
        Transform::from_xyz(x, 1.5, z),
    ));
}

fn animate_player(
    lib: Res<AnimationLibrary>,
    mut anim_players: Query<&mut AnimationPlayer, With<AnimationSetupDone>>,
    velocities: Query<&LinearVelocity, With<Player>>,
) {
    let Ok(mut player) = anim_players.single_mut() else { return; };
    let Ok(velocity) = velocities.single() else { return; };

    let vel = velocity.0;

    let target_index = if vel.length() < 0.1 {
        lib.idle_gun
    } else if vel.z.abs() > vel.x.abs() {
        if vel.z < 0.0 { lib.run } else { lib.run_back }
    } else if vel.x < 0.0 {
        lib.run_left
    } else {
        lib.run_right
    };

    if !player.is_playing_animation(target_index) {
        player.play(target_index).repeat();
    }
}

fn setup_animation(
    mut commands: Commands,
    lib: Option<Res<AnimationLibrary>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<AnimationSetupDone>>,
) {
    let Some(lib) = lib else { return; };
    for (entity, mut player) in &mut players {
        commands.entity(entity).insert((
            AnimationGraphHandle(lib.graph.clone()),
            AnimationSetupDone,
        ));
        player.play(lib.idle_gun).repeat();
    }
}
