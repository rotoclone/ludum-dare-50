use std::time::Duration;

use bevy_asset_loader::{AssetCollection, AssetLoader};
use bevy_kira_audio::{Audio, AudioChannel, AudioSource};
use bevy_rapier2d::{physics::JointHandleComponent, prelude::*};
use bevy_tweening::{
    component_animator_system,
    lens::{TransformPositionLens, TransformRotationLens},
    Animator, EaseFunction, Lens, Tracks, Tween, TweenCompleted, TweeningType,
};
use rand::Rng;

use crate::*;

const FADE_IN_TIME: Duration = Duration::from_secs(5);
const FADE_OUT_TIME: Duration = Duration::from_secs(5);
const VIBRATE_TIME: Duration = Duration::from_millis(500);
const VIBRATION_DELAY_SECONDS: f32 = 1.5;
const MISS_PENALTY_SECONDS: f32 = 1.0;

const FADE_OUT_TWEEN_COMPLETED: u64 = 1;
const FADE_IN_TWEEN_COMPLETED: u64 = 2;
const VIBRATE_TWEEN_COMPLETED: u64 = 3;

const ALARM_SOUND: &str = "sounds/alarm.ogg";
const HIT_SOUND: &str = "sounds/hit.ogg";
const DROP_SOUND: &str = "sounds/drop_2.ogg";

const ALARM_CHANNEL: &str = "alarm";

const MAX_VIBRATE_TRANSLATION: f32 = 100.0;
const MAX_VIBRATE_ROTATION: f32 = 0.75;

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

const ARM_ANCHOR_STARTING_POSITION_X: f32 = 1400.0;
const ARM_ANCHOR_STARTING_POSITION_Y: f32 = 0.0;
const ARM_ANCHOR_STARTING_POSITION_Z: f32 = 0.0;

const TABLE_EDGE_LEFT: f32 = -577.0;
const TABLE_EDGE_RIGHT: f32 = 440.0;
const TABLE_EDGE_TOP: f32 = 370.0;
const TABLE_EDGE_BOTTOM: f32 = -290.0;

const ROTATE_HAND_UP_KEY: KeyCode = KeyCode::W;
const ROTATE_HAND_DOWN_KEY: KeyCode = KeyCode::S;
const ROTATE_ARM_UP_KEY: KeyCode = KeyCode::Up;
const ROTATE_ARM_DOWN_KEY: KeyCode = KeyCode::Down;
const EXTEND_ARM_KEY: KeyCode = KeyCode::Left;
const RETRACT_ARM_KEY: KeyCode = KeyCode::Right;
const PRESS_KEY: KeyCode = KeyCode::Space;

const SNOOZE_MINUTES: u16 = 7;
const MINUTES_PER_HOUR: u16 = 60;
const HOURS_PER_DAY: u16 = 24;

const STARTING_TIME: GameTime = GameTime { hour: 8, minute: 0 };

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        AssetLoader::new(GameState::GameLoading)
            .continue_to_state(GameState::Game)
            .with_collection::<ImageAssets>()
            .with_collection::<AudioAssets>()
            .build(app);

        app.add_system_set(
            SystemSet::on_enter(GameState::Game)
                .with_system(game_setup)
                .with_system(alarm_sound_system),
        )
        .add_system_set(
            SystemSet::on_exit(GameState::Game)
                .with_system(despawn_components_system::<GameComponent>),
        )
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierRenderPlugin) //TODO
        .insert_resource(RapierConfiguration {
            gravity: Vector::zeros(),
            ..Default::default()
        })
        .add_event::<FadeEvent>()
        .add_event::<SnoozeEvent>()
        .add_event::<TweenCompleted>()
        .insert_resource(STARTING_TIME)
        .insert_resource(ValidPressPosition(false))
        .insert_resource(InputAllowed(true))
        .insert_resource(AlarmActive(true))
        .insert_resource(VibrateTimer(Timer::from_seconds(
            VIBRATION_DELAY_SECONDS,
            true,
        )))
        .insert_resource(MissTimer(Timer::from_seconds(MISS_PENALTY_SECONDS, false)))
        .insert_resource(NumSnoozes(0))
        .add_system(component_animator_system::<UiColor>)
        .add_system(fade_system)
        .add_system(hand_rotation_system)
        .add_system(arm_rotation_system)
        .add_system(arm_extension_system)
        .add_system(valid_press_position_system)
        .add_system(press_system)
        .add_system(sleep_system)
        .add_system(vibration_system)
        .add_system(table_bounds_system)
        .add_system(snooze_system)
        .add_system(miss_penalty_system);
    }

    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

#[derive(AssetCollection)]
struct AudioAssets {
    #[asset(path = "sounds/alarm.ogg")]
    alarm: Handle<AudioSource>,
    #[asset(path = "sounds/hit.ogg")]
    hit: Handle<AudioSource>,
    #[asset(path = "sounds/drop_2.ogg")]
    drop: Handle<AudioSource>,
}

#[derive(AssetCollection)]
struct ImageAssets {
    #[asset(path = "images/hand_transparent_2.png")]
    hand: Handle<Image>,
    #[asset(path = "images/arm_transparent.png")]
    arm: Handle<Image>,
    #[asset(path = "images/phone_transparent.png")]
    phone: Handle<Image>,
    #[asset(path = "images/background.png")]
    background: Handle<Image>,
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

#[derive(Component)]
struct Phone;

#[derive(Component)]
struct TimeDisplay;

#[derive(Component)]
struct SnoozeButton;

#[derive(Component)]
struct TouchArea;

struct ValidPressPosition(bool);

struct InputAllowed(bool);

struct AlarmActive(bool);

struct VibrateTimer(Timer);

struct MissTimer(Timer);

struct NumSnoozes(u32);

struct GameTime {
    hour: u16,
    minute: u16,
}

impl GameTime {
    /// Advances the time for a snooze
    fn snooze(&mut self) {
        let new_minute = (self.minute + SNOOZE_MINUTES) % MINUTES_PER_HOUR;
        let new_hour = if new_minute < self.minute {
            (self.hour + 1) % HOURS_PER_DAY
        } else {
            self.hour
        };

        self.minute = new_minute;
        self.hour = new_hour;

        println!("Advanced time to {self}"); //TODO
    }
}

impl std::fmt::Display for GameTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (converted_hour, am_or_pm) = if self.hour > 12 {
            (self.hour - 12, "PM")
        } else {
            (self.hour, "AM")
        };

        write!(f, "{converted_hour}:{:02} {am_or_pm}", self.minute)
    }
}

struct FadeEvent(FadeDirection);

enum FadeDirection {
    In,
    Out,
}

struct SnoozeEvent;

/// Sets up the main game screen.
fn game_setup(
    mut commands: Commands,
    image_assets: Res<ImageAssets>,
    font_assets: Res<FontAssets>,
    time: Res<GameTime>,
    mut event_writer: EventWriter<FadeEvent>,
) {
    // spawn overlay
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..Default::default()
            },
            color: OVERLAY_COLOR.into(),
            ..Default::default()
        })
        .insert(GameComponent)
        .insert(Overlay);

    // spawn background
    let background_position = Vec3::new(0.0, 0.0, 0.0);
    let background_scale = Vec3::ONE;
    commands
        .spawn_bundle(SpriteBundle {
            texture: image_assets.background.clone(),
            transform: Transform {
                translation: background_position,
                scale: background_scale,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GameComponent);

    // spawn phone
    let phone_position = Vec3::new(0.0, 0.0, 1.0);
    let phone_scale = Vec3::new(0.5, 0.5, 1.0);
    commands
        .spawn_bundle(SpriteBundle {
            texture: image_assets.phone.clone(),
            transform: Transform {
                translation: phone_position,
                scale: phone_scale,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GameComponent)
        .insert(Phone)
        .with_children(|parent| {
            // time display
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        time.to_string(),
                        TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 100.0,
                            color: Color::WHITE,
                        },
                        TextAlignment {
                            horizontal: HorizontalAlign::Center,
                            ..Default::default()
                        },
                    ),
                    transform: Transform {
                        translation: Vec3::new(0.0, 300.0, 1.0),
                        scale: Vec3::new(1.0, 1.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(TimeDisplay);

            // snooze button
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::RED,
                        custom_size: Some(Vec2::new(250.0, 100.0)),
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(0.0, -200.0, 1.0),
                        scale: Vec3::new(1.0, 1.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(SnoozeButton)
                .with_children(|parent| {
                    parent.spawn_bundle(Text2dBundle {
                        text: Text::with_section(
                            "SNOOZE",
                            TextStyle {
                                font: font_assets.main.clone(),
                                font_size: 60.0,
                                color: Color::BLACK,
                            },
                            TextAlignment {
                                horizontal: HorizontalAlign::Center,
                                vertical: VerticalAlign::Center,
                            },
                        ),
                        transform: Transform {
                            translation: Vec3::new(0.0, 0.0, 1.0),
                            scale: Vec3::new(1.0, 1.0, 1.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
        });

    // spawn arm anchor
    let arm_anchor_position = Vec3::new(
        ARM_ANCHOR_STARTING_POSITION_X,
        ARM_ANCHOR_STARTING_POSITION_Y,
        ARM_ANCHOR_STARTING_POSITION_Z,
    );
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
        .insert(GameComponent)
        .insert(ArmAnchor)
        .id();

    // spawn arm
    let arm_position = Vec3::new(500.0, 0.0, 10.0);
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
        .insert(GameComponent)
        .insert(Arm)
        .id();

    // spawn hand
    let hand_position = Vec3::new(0.0, 0.0, 11.0);
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
        .insert(GameComponent)
        .insert(Hand)
        .with_children(|parent| {
            // thumb
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::NONE,
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(-170.0, -45.0, 1.0),
                        scale: Vec3::new(30.0, 25.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(TouchArea);

            // index finger
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::NONE,
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(-160.0, 80.0, 1.0),
                        scale: Vec3::new(30.0, 25.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(TouchArea);

            // middle finger
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::NONE,
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(-135.0, 138.0, 1.0),
                        scale: Vec3::new(30.0, 25.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(TouchArea);

            // ring finger
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::NONE,
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(-42.0, 155.0, 1.0),
                        scale: Vec3::new(30.0, 25.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(TouchArea);

            // pinky
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::NONE,
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(60.0, 140.0, 1.0),
                        scale: Vec3::new(27.0, 22.0, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(TouchArea);
        })
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
                    FADE_IN_TWEEN_COMPLETED,
                ),
                FadeDirection::Out => fade_ui_color(
                    &mut commands,
                    entity,
                    Color::NONE,
                    OVERLAY_COLOR,
                    FADE_OUT_TIME,
                    FADE_OUT_TWEEN_COMPLETED,
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
    user_data: u64,
) {
    let tween = Tween::new(
        EaseFunction::SineInOut,
        TweeningType::Once,
        duration,
        UiColorLens {
            start: start_color.into(),
            end: end_color.into(),
        },
    )
    .with_completed_event(true, user_data);
    commands.entity(entity).insert(Animator::new(tween));
}

/// Handles rotating the hand
fn hand_rotation_system(
    input_allowed: Res<InputAllowed>,
    keyboard: Res<Input<KeyCode>>,
    mut joint_set: ResMut<ImpulseJointSet>,
    mut query: Query<(&JointHandleComponent, &mut RigidBodyActivationComponent), With<Hand>>,
) {
    for (joint_handle, mut activation) in query.iter_mut() {
        let joint = joint_set
            .get_mut(joint_handle.handle())
            .expect("couldn't find joint");
        activation.wake_up(true);

        if keyboard.pressed(ROTATE_HAND_DOWN_KEY) && input_allowed.0 {
            joint.data =
                joint
                    .data
                    .motor_velocity(JointAxis::AngX, HAND_CONTROL_POWER, HAND_MOTOR_FACTOR);
        } else if keyboard.pressed(ROTATE_HAND_UP_KEY) && input_allowed.0 {
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
    input_allowed: Res<InputAllowed>,
    keyboard: Res<Input<KeyCode>>,
    mut joint_set: ResMut<ImpulseJointSet>,
    mut query: Query<(&JointHandleComponent, &mut RigidBodyActivationComponent), With<Arm>>,
) {
    for (joint_handle, mut activation) in query.iter_mut() {
        let joint = joint_set
            .get_mut(joint_handle.handle())
            .expect("couldn't find joint");
        activation.wake_up(true);

        if keyboard.pressed(ROTATE_ARM_DOWN_KEY) && input_allowed.0 {
            joint.data =
                joint
                    .data
                    .motor_velocity(JointAxis::AngX, ARM_CONTROL_POWER, ARM_MOTOR_FACTOR);
        } else if keyboard.pressed(ROTATE_ARM_UP_KEY) && input_allowed.0 {
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
    input_allowed: Res<InputAllowed>,
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
        if keyboard.pressed(EXTEND_ARM_KEY)
            && position.position.translation.x > ARM_EXTENSION_LIMIT
            && input_allowed.0
        {
            activation.wake_up(true);
            velocity.linvel = Vec2::new(-ARM_EXTENSION_CONTROL_POWER, 0.0).into();
        } else if keyboard.pressed(RETRACT_ARM_KEY)
            && position.position.translation.x < ARM_RETRACTION_LIMIT
            && input_allowed.0
        {
            activation.wake_up(true);
            velocity.linvel = Vec2::new(ARM_EXTENSION_CONTROL_POWER, 0.0).into();
        } else {
            velocity.linvel = Vec2::new(0.0, 0.0).into();
        }
    }
}

/// Determines whether a finger is in the correct position to press snooze
fn valid_press_position_system(
    mut valid_press_position: ResMut<ValidPressPosition>,
    snooze_button_query: Query<(&GlobalTransform, &Sprite), With<SnoozeButton>>,
    touch_area_query: Query<&GlobalTransform, With<TouchArea>>,
) {
    for (snooze_transform, snooze_sprite) in snooze_button_query.iter() {
        for touch_area_transform in touch_area_query.iter() {
            if intersects(
                snooze_transform,
                snooze_sprite.custom_size,
                touch_area_transform,
                None,
            ) {
                valid_press_position.0 = true;
                return;
            }
        }
    }

    valid_press_position.0 = false;
}

/// Determines whether 2 transforms intersect
fn intersects(
    a: &GlobalTransform,
    a_sprite_custom_size: Option<Vec2>,
    b: &GlobalTransform,
    b_sprite_custom_size: Option<Vec2>,
) -> bool {
    let a_width = a_sprite_custom_size.unwrap_or(Vec2::ONE).x * a.scale.x;
    let a_height = a_sprite_custom_size.unwrap_or(Vec2::ONE).y * a.scale.y;
    let a_left = a.translation.x - (a_width / 2.0);
    let a_right = a.translation.x + (a_width / 2.0);
    let a_top = a.translation.y + (a_height / 2.0);
    let a_bottom = a.translation.y - (a_height / 2.0);

    let b_width = b_sprite_custom_size.unwrap_or(Vec2::ONE).x * b.scale.x;
    let b_height = b_sprite_custom_size.unwrap_or(Vec2::ONE).y * b.scale.y;
    let b_left = b.translation.x - (b_width / 2.0);
    let b_right = b.translation.x + (b_height / 2.0);
    let b_top = b.translation.y + (b.scale.y / 2.0);
    let b_bottom = b.translation.y - (b.scale.y / 2.0);

    a_left < b_right && a_right > b_left && a_top > b_bottom && a_bottom < b_top
}

/// Handles attempts to press the snooze button
fn press_system(
    mut input_allowed: ResMut<InputAllowed>,
    mut miss_timer: ResMut<MissTimer>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    keyboard: Res<Input<KeyCode>>,
    valid_press_position: Res<ValidPressPosition>,
    mut event_writer: EventWriter<SnoozeEvent>,
) {
    if !input_allowed.0 {
        return;
    }

    if keyboard.just_pressed(PRESS_KEY) {
        audio.play(asset_server.load(HIT_SOUND));
        if valid_press_position.0 {
            // gotcha
            println!("you pressed snooze"); //TODO
            event_writer.send(SnoozeEvent);
        } else {
            // and that's a bad miss
            println!("you missed"); //TODO
            input_allowed.0 = false;
            miss_timer.0 = Timer::from_seconds(MISS_PENALTY_SECONDS, false);
        }
    }
}

/// Handles re-enabling input once the miss penalty time has elapsed
fn miss_penalty_system(
    mut input_allowed: ResMut<InputAllowed>,
    mut miss_timer: ResMut<MissTimer>,
    time: Res<Time>,
) {
    if miss_timer.0.tick(time.delta()).just_finished() {
        input_allowed.0 = true;
    }
}

/// Handles when the snooze button is pressed
fn snooze_system(
    mut time: ResMut<GameTime>,
    mut num_snoozes: ResMut<NumSnoozes>,
    mut input_allowed: ResMut<InputAllowed>,
    mut alarm_active: ResMut<AlarmActive>,
    mut vibrate_timer: ResMut<VibrateTimer>,
    audio: Res<Audio>,
    mut event_reader: EventReader<SnoozeEvent>,
    mut event_writer: EventWriter<FadeEvent>,
) {
    if event_reader.iter().next().is_none() {
        // no snoozin
        return;
    }

    // stop playing alarm sound
    audio.stop_channel(&AudioChannel::new(ALARM_CHANNEL.to_string()));

    // disallow input
    input_allowed.0 = false;

    // turn off the alarm
    alarm_active.0 = false;

    // increment snooze counter
    num_snoozes.0 += 1;

    // update time
    time.snooze();

    if vibrate_timer.0.duration().as_secs_f32() > VIBRATE_TIME.as_secs_f32() {
        // a little bit faster now
        vibrate_timer.0 = Timer::from_seconds(vibrate_timer.0.duration().as_secs_f32() * 0.9, true);
    }

    // fade out
    event_writer.send(FadeEvent(FadeDirection::Out));
}

/// Handles updates while the player gets a few minutes of precious sleep
fn sleep_system(
    mut event_reader: EventReader<TweenCompleted>,
    mut event_writer: EventWriter<FadeEvent>,
    time: Res<GameTime>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    mut input_allowed: ResMut<InputAllowed>,
    mut alarm_active: ResMut<AlarmActive>,
    mut time_display_query: Query<&mut Text, With<TimeDisplay>>,
    mut arm_anchor_query: Query<&mut RigidBodyPositionComponent, With<ArmAnchor>>,
) {
    for event in event_reader.iter() {
        if event.user_data != FADE_OUT_TWEEN_COMPLETED {
            continue;
        }

        // update the time display
        for mut time_text in time_display_query.iter_mut() {
            time_text.sections[0].value = time.to_string();
        }

        // move the arm anchor back
        for mut position in arm_anchor_query.iter_mut() {
            position.position.translation.x = ARM_ANCHOR_STARTING_POSITION_X;
            position.position.translation.y = ARM_ANCHOR_STARTING_POSITION_Y;
        }

        //TODO rotate the arm back

        //TODO wait a few seconds

        // start playing alarm sound
        audio.play_looped_in_channel(
            asset_server.load(ALARM_SOUND),
            &AudioChannel::new(ALARM_CHANNEL.to_string()),
        );

        // allow input
        input_allowed.0 = true;

        // turn on the alarm
        alarm_active.0 = true;

        // fade in
        event_writer.send(FadeEvent(FadeDirection::In))
    }
}

/// Handles vibrating the phone around
fn vibration_system(
    mut commands: Commands,
    alarm_active: Res<AlarmActive>,
    time: Res<Time>,
    mut vibrate_timer: ResMut<VibrateTimer>,
    phone_query: Query<(Entity, &Transform), With<Phone>>,
) {
    if !alarm_active.0 {
        return;
    }

    if vibrate_timer.0.tick(time.delta()).finished() {
        for (entity, transform) in phone_query.iter() {
            vibrate_phone(
                &mut commands,
                entity,
                transform.translation,
                transform.rotation,
            );
        }
    }
}

/// Vibrates the phone to a random position
fn vibrate_phone(
    commands: &mut Commands,
    entity: Entity,
    start_position: Vec3,
    start_rotation: Quat,
) {
    let mut rng = rand::thread_rng();

    let end_x = rng.gen_range(
        (start_position.x - MAX_VIBRATE_TRANSLATION)..(start_position.x + MAX_VIBRATE_TRANSLATION),
    );
    let end_y = rng.gen_range(
        (start_position.y - MAX_VIBRATE_TRANSLATION)..(start_position.y + MAX_VIBRATE_TRANSLATION),
    );
    let end_position = Vec3::new(end_x, end_y, start_position.z);
    let position_tween = Tween::new(
        EaseFunction::SineInOut,
        TweeningType::Once,
        VIBRATE_TIME,
        TransformPositionLens {
            start: start_position,
            end: end_position,
        },
    )
    .with_completed_event(true, VIBRATE_TWEEN_COMPLETED);

    let end_rotation = rng.gen_range(
        (start_rotation.z - MAX_VIBRATE_ROTATION)..(start_rotation.z + MAX_VIBRATE_ROTATION),
    );
    let rotation_tween = Tween::new(
        EaseFunction::SineInOut,
        TweeningType::Once,
        VIBRATE_TIME,
        TransformRotationLens {
            start: start_rotation,
            end: Quat::from_rotation_z(end_rotation),
        },
    )
    .with_completed_event(true, VIBRATE_TWEEN_COMPLETED);

    commands
        .entity(entity)
        .insert(Animator::new(Tracks::new(vec![
            position_tween,
            rotation_tween,
        ])));
}

/// Handles checking to make sure the phone is still on the table
fn table_bounds_system(
    mut commands: Commands,
    time: Res<GameTime>,
    num_snoozes: Res<NumSnoozes>,
    audio: Res<Audio>,
    phone_query: Query<(Entity, &GlobalTransform), With<Phone>>,
    mut input_allowed: ResMut<InputAllowed>,
    mut alarm_active: ResMut<AlarmActive>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in phone_query.iter() {
        if transform.translation.x < TABLE_EDGE_LEFT
            || transform.translation.x > TABLE_EDGE_RIGHT
            || transform.translation.y > TABLE_EDGE_TOP
            || transform.translation.y < TABLE_EDGE_BOTTOM
        {
            // it fell off
            input_allowed.0 = false;
            alarm_active.0 = false;
            commands.entity(entity).despawn_recursive();
            audio.play(asset_server.load(DROP_SOUND));
            show_game_over_screen(&mut commands, time, num_snoozes, asset_server);
            return;
        }
    }
}

fn show_game_over_screen(
    commands: &mut Commands,
    time: Res<GameTime>,
    num_snoozes: Res<NumSnoozes>,
    asset_server: Res<AssetServer>,
) {
    let text = format!(
        "Your phone fell on the floor!\nYou got out of bed at {} after hitting snooze {} times",
        *time, num_snoozes.0
    );

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(80.0), Val::Percent(20.0)),
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Percent(40.0),
                    left: Val::Percent(10.0),
                    ..Default::default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..Default::default()
            },
            color: UiColor(Color::rgba(0.0, 0.0, 0.0, 0.7)),
            ..Default::default()
        })
        .insert(GameComponent)
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: text,
                        style: TextStyle {
                            font: asset_server.load(MAIN_FONT),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    }],
                    alignment: TextAlignment {
                        horizontal: HorizontalAlign::Center,
                        vertical: VerticalAlign::Center,
                    },
                },
                style: Style {
                    align_self: AlignSelf::Center,
                    ..Default::default()
                },
                ..Default::default()
            });
        });
}

fn alarm_sound_system(
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    alarm_active: Res<AlarmActive>,
) {
    if alarm_active.is_changed() {
        if alarm_active.0 {
            audio.play_looped_in_channel(
                asset_server.load(ALARM_SOUND),
                &AudioChannel::new(ALARM_CHANNEL.to_string()),
            );
        } else {
            //TODO this doesn't seem to do anything
            audio.stop_channel(&AudioChannel::new(ALARM_CHANNEL.to_string()));
        }
    }
}
