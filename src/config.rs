use bevy::feathers::controls::{
    FeathersButton, FeathersMenu, FeathersMenuButton, FeathersMenuItem, FeathersMenuPopup,
};
use bevy::feathers::theme::ThemeBackgroundColor;
use bevy::feathers::*;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;

use crate::image_selection::{self, PreviewImage, SelectedImagePath};
use crate::microphone::{
    CurrentMicrophoneText, MicrophoneDevices, MicrophoneLevelText, MicrophoneOption,
    select_microphone,
};

pub(crate) struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let microphones = app
            .world()
            .resource::<MicrophoneDevices>()
            .devices
            .iter()
            .map(|microphone| microphone.name.clone())
            .collect::<Vec<_>>();
        let selected_microphone = (!microphones.is_empty()).then_some(0);

        app.add_systems(
            Startup,
            (move || config_scene(microphones.clone(), selected_microphone)).spawn(),
        );
    }
}

fn config_scene(microphones: Vec<String>, selected_microphone: Option<usize>) -> impl SceneList {
    bsn_list![Camera2d, config_window(microphones, selected_microphone)]
}

fn config_window(microphones: Vec<String>, selected_microphone: Option<usize>) -> impl Scene {
    let selected_name = selected_microphone
        .and_then(|index| microphones.get(index))
        .cloned()
        .unwrap_or_else(|| "No microphone found".to_string());

    bsn![
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            (
                @FeathersButton {
                    @caption: bsn! { Text("Select image") }
                }
                on(image_selection::begin_file_selection)
            ),
            Text("Microphone input"),
            (
                @FeathersMenu
                Children [
                    (
                        @FeathersMenuButton {
                            @caption: bsn! { (Text(selected_name) CurrentMicrophoneText) }
                        }
                    ),
                    (
                        @FeathersMenuPopup
                        Children [
                            {microphone_menu_items(microphones)}
                        ]
                    )
                ]
            ),
            (Text("Level: -- dBFS") MicrophoneLevelText),
            (
                Node {
                    max_width: px(320),
                    max_height: px(320),
                }
                ImageNode {}
                PreviewImage
                SelectedImagePath::default()
                Visibility::Hidden
            )
        ]
    ]
}

fn microphone_menu_items(microphones: Vec<String>) -> impl SceneList {
    microphones
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            bsn![
                (
                    @FeathersMenuItem {
                        @caption: bsn! { Text(name) }
                    }
                    MicrophoneOption { index }
                    on(select_microphone)
                )
            ]
        })
        .collect::<Vec<_>>()
}
