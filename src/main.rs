use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_asset_loader::{AssetCollection, AssetLoader};
use bevy_inspector_egui::{WorldInspectorParams, WorldInspectorPlugin};

mod menu;
use bevy_kira_audio::AudioPlugin;
use bevy_tweening::TweeningPlugin;
use menu::*;

mod game;
use game::*;

mod game_over;
use game_over::*;

const DEV_MODE: bool = false;

const MAIN_FONT: &str = "fonts/FiraMono-Medium.ttf";

const NORMAL_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const HOVERED_BUTTON: Color = Color::rgb(0.35, 0.35, 0.35);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    Menu,
    GameLoading,
    Game,
    GameOver,
}

#[derive(AssetCollection)]
struct FontAssets {
    #[asset(path = "fonts/FiraMono-Medium.ttf")]
    main: Handle<Font>,
}

/// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_components_system<T: Component>(
    to_despawn: Query<Entity, With<T>>,
    mut commands: Commands,
) {
    despawn_components(to_despawn, &mut commands);
}

fn despawn_components<T: Component>(to_despawn: Query<Entity, With<T>>, commands: &mut Commands) {
    for entity in to_despawn.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup(mut commands: Commands) {
    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

type InteractedButtonTuple = (Changed<Interaction>, With<Button>);

/// Handles changing button colors when they're interacted with.
fn button_color_system(
    mut interaction_query: Query<(&Interaction, &mut UiColor), InteractedButtonTuple>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        *color = match *interaction {
            Interaction::Clicked => PRESSED_BUTTON.into(),
            Interaction::Hovered => HOVERED_BUTTON.into(),
            Interaction::None => NORMAL_BUTTON.into(),
        }
    }
}

/// Handles showing the world inspector.
fn world_inspector_system(
    keyboard: Res<Input<KeyCode>>,
    mut inspector_params: ResMut<WorldInspectorParams>,
) {
    if keyboard.pressed(KeyCode::Equals) {
        inspector_params.enabled = true;
    }
}

fn main() {
    let mut app = App::new();
    AssetLoader::new(GameState::Menu)
        .with_collection::<FontAssets>()
        .build(&mut app);
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            title: "Snooze".to_string(),
            width: 1280.0,
            height: 720.0,
            ..Default::default()
        })
        .add_state(GameState::Menu)
        .add_startup_system(setup)
        .add_plugin(MenuPlugin)
        .add_plugin(GamePlugin)
        .add_plugin(GameOverPlugin)
        .add_system(button_color_system)
        .add_plugins(DefaultPlugins)
        .add_plugin(AudioPlugin)
        .add_plugin(TweeningPlugin);

    if DEV_MODE {
        app.add_system(bevy::input::system::exit_on_esc_system)
            .add_system(world_inspector_system)
            .add_plugin(LogDiagnosticsPlugin::default())
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(WorldInspectorPlugin::new())
            .insert_resource(WorldInspectorParams {
                enabled: false,
                ..Default::default()
            });
    }

    app.run();
}
