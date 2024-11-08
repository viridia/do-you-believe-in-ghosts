//! Example which uses states and a switch view.

use bevy::{color::palettes::css, prelude::*, ui};
use do_you_believe::{EffectPlugin, Switch, WithChildren};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Play,
    Pause,
    Intro,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            EffectPlugin,
        ))
        .insert_state(GameState::Intro)
        .add_systems(Startup, setup_view_root)
        .add_systems(Update, (close_on_esc, handle_key_input))
        .run();
}

fn setup_view_root(mut commands: Commands) {
    let camera = commands.spawn((Camera::default(), Camera2d)).id();

    commands.spawn((
        Node {
            left: ui::Val::Px(0.),
            top: ui::Val::Px(0.),
            right: ui::Val::Px(0.),
            // bottom: ui::Val::Px(0.),
            position_type: ui::PositionType::Absolute,
            display: ui::Display::Flex,
            flex_direction: ui::FlexDirection::Row,
            border: ui::UiRect::all(ui::Val::Px(3.)),
            ..default()
        },
        BorderColor(css::ALICE_BLUE.into()),
        TargetCamera(camera),
        WithChildren((
            Text::new("Game State: "),
            Switch::new(|state: Res<State<GameState>>| state.get().clone())
                .case(GameState::Intro, || (Text::new("Intro"),))
                .case(GameState::Pause, || (Text::new("Paused"),))
                .fallback(|| (Text::new("Playing"),))
                .build(),
            (
                Node {
                    border: ui::UiRect::all(ui::Val::Px(3.)),
                    ..default()
                },
                BorderColor(css::LIME.into()),
                WithChildren((Text::new("!!"),)),
            ),
        )),
    ));
}

// struct StateName;

// impl UiTemplate for StateName {
//     fn build(&self, builder: &mut UiBuilder) {
//         builder.switch(
//             |rcx: &Rcx| rcx.read_resource::<State<GameState>>().get().clone(),
//             |builder| {
//                 builder
//                     .case(GameState::Intro, |builder| {
//                         builder.text("Intro");
//                     })
//                     .case(GameState::Pause, |builder| {
//                         builder.text("Pause");
//                     })
//                     .fallback(|builder| {
//                         builder.text("Play");
//                     });
//             },
//         );
//     }
// }

fn handle_key_input(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if key.just_pressed(KeyCode::Space) {
        match state.get() {
            GameState::Intro => next_state.set(GameState::Play),
            GameState::Play => next_state.set(GameState::Pause),
            GameState::Pause => next_state.set(GameState::Play),
        }
    }
}

pub fn close_on_esc(input: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}
