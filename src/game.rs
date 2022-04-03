use std::time::Duration;

use bevy_asset_loader::{AssetCollection, AssetLoader};
use bevy_rapier2d::{physics::JointHandleComponent, prelude::*};
use bevy_tweening::{component_animator_system, Animator, EaseFunction, Lens, Tween, TweeningType};

use crate::*;

const FADE_IN_TIME: Duration = Duration::from_secs(5);
const FADE_OUT_TIME: Duration = Duration::from_secs(5);
const OVERLAY_COLOR: Color = Color::BLACK;
const HAND_CONTROL_POWER: f32 = 2.0;
const ARM_CONTROL_POWER: f32 = 1.0;
const ARM_EXTENSION_CONTROL_POWER: f32 = 150.0;
const LINEAR_DAMPING: f32 = 1.0;
const ANGULAR_DAMPING: f32 = 1.0;
const HAND_MOTOR_FACTOR: f32 = 0.1;
const ARM_MOTOR_FACTOR: f32 = 0.05;

const ARM_EXTENSION_LIMIT: f32 = 750.0;
const ARM_RETRACTION_LIMIT: f32 = 1700.0;

const ROTATE_HAND_UP_KEY: KeyCode = KeyCode::W;
const ROTATE_HAND_DOWN_KEY: KeyCode = KeyCode::S;
const ROTATE_ARM_UP_KEY: KeyCode = KeyCode::Up;
const ROTATE_ARM_DOWN_KEY: KeyCode = KeyCode::Down;
const EXTEND_ARM_KEY: KeyCode = KeyCode::Left;
const RETRACT_ARM_KEY: KeyCode = KeyCode::Right;
const PRESS_KEY: KeyCode = KeyCode::Space;

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
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugin(RapierRenderPlugin) //TODO
            .insert_resource(RapierConfiguration {
                gravity: Vector::zeros(),
                ..Default::default()
            })
            .add_system(component_animator_system::<UiColor>)
            .add_system(fade_system)
            .add_system(hand_control_system)
            .add_system(arm_rotation_system)
            .add_system(arm_extension_system);
    }

    fn name(&self) -> &str {
        std::any::type_name::<Self>()
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

#[derive(Component)]
struct ArmAnchor;

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

    // spawn arm anchor
    let arm_anchor_position = Vec3::new(1400.0, 0.0, 0.0);
    let arm_anchor = commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                ..Default::default()
            },
            transform: Transform {
                translation: arm_anchor_position,
                scale: Vec3::new(10.0, 10.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            position: arm_anchor_position.into(),
            damping: RigidBodyDamping {
                linear_damping: LINEAR_DAMPING,
                angular_damping: ANGULAR_DAMPING,
            }
            .into(),
            body_type: RigidBodyType::KinematicVelocityBased.into(),
            mass_properties: RigidBodyMassPropsFlags::ROTATION_LOCKED.into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::ball(10.0).into(),
            collider_type: ColliderType::Sensor.into(),
            mass_properties: ColliderMassProps::Density(1.0).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        //TODO .insert(ColliderDebugRender::with_id(0))
        .insert(ArmAnchor)
        .id();

    // spawn arm
    let arm_position = Vec3::new(500.0, 0.0, 1.0);
    let arm_rotation = Quat::from_rotation_z(-0.79);
    let arm_scale = Vec3::ONE;
    let arm = commands
        .spawn_bundle(SpriteBundle {
            texture: image_assets.arm.clone(),
            transform: Transform {
                translation: arm_position,
                rotation: arm_rotation,
                scale: arm_scale,
            },
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            position: (arm_position, arm_rotation).into(),
            damping: RigidBodyDamping {
                linear_damping: LINEAR_DAMPING,
                angular_damping: ANGULAR_DAMPING,
            }
            .into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::ball(100.0).into(),
            collider_type: ColliderType::Sensor.into(),
            mass_properties: ColliderMassProps::Density(1.0).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        //TODO .insert(ColliderDebugRender::with_id(1))
        .insert(Arm)
        .id();

    // spawn hand
    let hand_position = Vec3::new(0.0, 0.0, 2.0);
    let hand_rotation = Quat::from_rotation_z(0.0);
    let hand_scale = Vec3::ONE;
    let hand = commands
        .spawn_bundle(SpriteBundle {
            texture: image_assets.hand.clone(),
            transform: Transform {
                translation: hand_position,
                rotation: hand_rotation,
                scale: hand_scale,
            },
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            position: (hand_position, hand_rotation).into(),
            damping: RigidBodyDamping {
                linear_damping: LINEAR_DAMPING,
                angular_damping: ANGULAR_DAMPING,
            }
            .into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::ball(100.0).into(),
            mass_properties: ColliderMassProps::Density(1.0).into(),
            ..Default::default()
        })
        .insert(ColliderPositionSync::Discrete)
        //TODO .insert(ColliderDebugRender::with_id(2))
        .insert(Hand)
        .id();

    // attach arm to arm anchor
    let arm_joint = RevoluteJoint::new()
        .local_anchor1(point![-50.0, 0.0])
        .local_anchor2(point![300.0, -250.0])
        .motor_model(MotorModel::VelocityBased)
        .motor_velocity(0.0, ARM_MOTOR_FACTOR);
    commands
        .entity(arm)
        .insert(JointBuilderComponent::new(arm_joint, arm_anchor, arm));

    // attach hand to arm
    let hand_joint = RevoluteJoint::new()
        .local_anchor1(point![-300.0, 250.0])
        .local_anchor2(point![130.0, -120.0])
        .motor_model(MotorModel::VelocityBased)
        .motor_velocity(0.0, HAND_MOTOR_FACTOR);
    commands
        .entity(hand)
        .insert(JointBuilderComponent::new(hand_joint, arm, hand));

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
    mut joint_set: ResMut<ImpulseJointSet>,
    mut query: Query<(&JointHandleComponent, &mut RigidBodyActivationComponent), With<Hand>>,
) {
    for (joint_handle, mut activation) in query.iter_mut() {
        let joint = joint_set
            .get_mut(joint_handle.handle())
            .expect("couldn't find joint");

        if keyboard.pressed(ROTATE_HAND_DOWN_KEY) {
            activation.wake_up(true);
            joint.data =
                joint
                    .data
                    .motor_velocity(JointAxis::AngX, HAND_CONTROL_POWER, HAND_MOTOR_FACTOR);
        } else if keyboard.pressed(ROTATE_HAND_UP_KEY) {
            activation.wake_up(true);
            joint.data =
                joint
                    .data
                    .motor_velocity(JointAxis::AngX, -HAND_CONTROL_POWER, HAND_MOTOR_FACTOR);
        } else {
            joint.data = joint
                .data
                .motor_velocity(JointAxis::AngX, 0.0, HAND_MOTOR_FACTOR);
        }
    }
}

/// Handles rotating the arm
fn arm_rotation_system(
    keyboard: Res<Input<KeyCode>>,
    mut joint_set: ResMut<ImpulseJointSet>,
    mut query: Query<(&JointHandleComponent, &mut RigidBodyActivationComponent), With<Arm>>,
) {
    for (joint_handle, mut activation) in query.iter_mut() {
        let joint = joint_set
            .get_mut(joint_handle.handle())
            .expect("couldn't find joint");

        if keyboard.pressed(ROTATE_ARM_DOWN_KEY) {
            activation.wake_up(true);
            joint.data =
                joint
                    .data
                    .motor_velocity(JointAxis::AngX, ARM_CONTROL_POWER, ARM_MOTOR_FACTOR);
        } else if keyboard.pressed(ROTATE_ARM_UP_KEY) {
            activation.wake_up(true);
            joint.data =
                joint
                    .data
                    .motor_velocity(JointAxis::AngX, -ARM_CONTROL_POWER, ARM_MOTOR_FACTOR);
        } else {
            joint.data = joint
                .data
                .motor_velocity(JointAxis::AngX, 0.0, ARM_MOTOR_FACTOR);
        }
    }
}

/// Handles extending and retracting the arm
fn arm_extension_system(
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<
        (
            &mut RigidBodyVelocityComponent,
            &RigidBodyPositionComponent,
            &mut RigidBodyActivationComponent,
        ),
        With<ArmAnchor>,
    >,
) {
    for (mut velocity, position, mut activation) in query.iter_mut() {
        if keyboard.pressed(EXTEND_ARM_KEY) && position.position.translation.x > ARM_EXTENSION_LIMIT
        {
            activation.wake_up(true);
            velocity.linvel = Vec2::new(-ARM_EXTENSION_CONTROL_POWER, 0.0).into();
        } else if keyboard.pressed(RETRACT_ARM_KEY)
            && position.position.translation.x < ARM_RETRACTION_LIMIT
        {
            activation.wake_up(true);
            velocity.linvel = Vec2::new(ARM_EXTENSION_CONTROL_POWER, 0.0).into();
        } else {
            velocity.linvel = Vec2::new(0.0, 0.0).into();
        }
    }
}
