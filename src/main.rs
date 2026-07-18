use bevy::feathers::controls::FeathersButton;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::{ThemeBackgroundColor, UiTheme};
use bevy::feathers::*;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, config_window()]
}

fn config_window() -> impl Scene {
    bsn![Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[(
            @FeathersButton{
                @caption: bsn!{ Text("Test")}
            }
            Node{
                flex_grow: 1.0,
                margin: UiRect::horizontal(percent(2.0))
            }
            on(|_activate: On<Activate>| {
                info!("Button activated");
            })
    )    ]
        ]
}
