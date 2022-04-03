use std::time::Duration;

use bevy_asset_loader::{AssetCollection, AssetLoader};
use bevy_tweening::{component_animator_system, Animator, EaseFunction, Lens, Tween, TweeningType};
use heron::{prelude::*};

use crate::*;

const FADE_IN_TIME: Duration = Duration::from_secs(5);
const FADE_OUT_TIME: Duration = Duration::from_secs(5);
const OVERLAY_COLOR: Color = Color::BLACK;
const CONTROL_POWER: f32 = 7.0;
const LINEAR_DAMPING: f32 = 1.0;
const ANGULAR_DAMPING: f32 = 1.0;

const ROTATE_HAND_LEFT_KEY: KeyCode = KeyCode::Left;
const ROTATE_HAND_RIGHT_KEY: KeyCode = KeyCode::Right;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        AssetLoader::new(GameState::GameLoading)
            .continue_to_state(GameState::Game)
            .with_collection::<ImageAssets>()
            //TODO .with_collection::<AudioAssets>()
            .build(app);

        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(game_setup))
            .add_system_set(
                SystemSet::on_exit(GameState::Game)
                    .with_system(despawn_components_system::<GameComponent>),
            )
            .add_event::<FadeEvent>()
            .add_plugin(PhysicsPlugin::default())
            .add_system(component_animator_system::<UiColor>)
            .add_system(fade_system)
            .add_system(hand_control_system);
    }
}

#[derive(AssetCollection)]
struct AudioAssets {
    #[asset(path = "sounds/alarm.ogg")]
    alarm: Handle<AudioSource>,
}

#[derive(AssetCollection)]
struct ImageAssets {
    #[asset(path = "images/hand_transparent.png")]
    hand: Handle<Image>,
    #[asset(path = "images/arm_transparent.png")]
    arm: Handle<Image>,
}

#[derive(Component)]
struct GameComponent;

#[derive(Component)]
struct GameLoadingComponent;

#[derive(Component)]
struct Overlay;

#[derive(Component)]
struct Hand;

#[derive(Component)]
struct Arm;

struct FadeEvent(FadeDirection);

enum FadeDirection {
    In,
    Out,
}

/// Sets up the main game screen.
fn game_setup(
    mut commands: Commands,
    image_assets: Res<ImageAssets>,
    mut event_writer: EventWriter<FadeEvent>,
) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..Default::default()
            },
            color: OVERLAY_COLOR.into(),
            ..Default::default()
        })
        .insert(Overlay);

    let arm_position = Vec3::new(200.0, -200.0, 1.0);
    let arm_scale = Vec3::ONE;

    commands
        .spawn_bundle(SpriteBundle {
            texture: image_assets.arm.clone(),
            transform: Transform {
                translation: arm_position,
                scale: arm_scale,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Arm);

    let hand_position = Vec3::new(0.0, 0.0, 2.0);
    let hand_scale = Vec3::ONE;

    commands
        .spawn_bundle(SpriteBundle {
            texture: image_assets.hand.clone(),
            transform: Transform {
                translation: hand_position,
                scale: hand_scale,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Hand)
        .insert(RigidBody::Dynamic)
        .insert(CollisionShape::Sphere { radius: 10.0 })
        .insert(Velocity::from_linear(Vec3::ZERO))
        .insert(Acceleration::from_linear(Vec3::ZERO))
        .insert(Damping::from_linear(LINEAR_DAMPING).with_angular(ANGULAR_DAMPING))
        .insert(PhysicMaterial {
            friction: 1.0,
            density: 10.0,
            ..Default::default()
        });

    event_writer.send(FadeEvent(FadeDirection::In));
}

/// Handles fading in and out
fn fade_system(
    mut commands: Commands,
    mut events: EventReader<FadeEvent>,
    query: Query<Entity, With<Overlay>>,
) {
    for event in events.iter() {
        for entity in query.iter() {
            match event.0 {
                FadeDirection::In => fade_ui_color(
                    &mut commands,
                    entity,
                    OVERLAY_COLOR,
                    Color::NONE,
                    FADE_IN_TIME,
                ),
                FadeDirection::Out => fade_ui_color(
                    &mut commands,
                    entity,
                    Color::NONE,
                    OVERLAY_COLOR,
                    FADE_OUT_TIME,
                ),
            }
        }
    }
}

struct UiColorLens {
    start: UiColor,
    end: UiColor,
}

impl Lens<UiColor> for UiColorLens {
    fn lerp(&mut self, target: &mut UiColor, ratio: f32) {
        // copied from SpriteColorLens
        // Note: Add<f32> for Color affects alpha, but not Mul<f32>. So use Vec4 for consistency.
        let start: Vec4 = self.start.0.into();
        let end: Vec4 = self.end.0.into();
        let value = start.lerp(end, ratio);
        target.0 = value.into();
    }
}

/// Fades the `UiColor` of an entity
fn fade_ui_color(
    commands: &mut Commands,
    entity: Entity,
    start_color: Color,
    end_color: Color,
    duration: Duration,
) {
    let tween = Tween::new(
        EaseFunction::SineInOut,
        TweeningType::Once,
        duration,
        UiColorLens {
            start: start_color.into(),
            end: end_color.into(),
        },
    );
    commands.entity(entity).insert(Animator::new(tween));
}

/// Handles moving the hand around
fn hand_control_system(
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<&mut Acceleration, With<Hand>>,
) {
    for mut accel in query.iter_mut() {
        if keyboard.pressed(ROTATE_HAND_LEFT_KEY) {
            accel.angular = AxisAngle::new(Vec3::Z, CONTROL_POWER);
        } else if keyboard.pressed(ROTATE_HAND_RIGHT_KEY) {
            accel.angular = AxisAngle::new(Vec3::Z, -CONTROL_POWER);
        } else {
            accel.angular = AxisAngle::new(Vec3::Z, 0.0);
        }

        if keyboard.pressed(KeyCode::Up) {
            accel.linear.y = CONTROL_POWER;
        } else if keyboard.pressed(KeyCode::Down) {
            accel.linear.y = -CONTROL_POWER;
        } else {
            accel.linear.y = 0.0;
        }
    }
}
