use bevy::feathers::controls::{
    FeathersButton, FeathersCheckbox, FeathersMenu, FeathersMenuButton, FeathersMenuItem,
    FeathersMenuPopup, FeathersSlider,
};
use bevy::feathers::theme::ThemeBackgroundColor;
use bevy::feathers::*;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{SliderPrecision, SliderStep};

use crate::image_selection;
use crate::microphone::{
    CurrentMicrophoneText, MicrophoneDevices, MicrophoneLevelText, MicrophoneOption,
    select_microphone,
};
use crate::puppet_window::{
    self, OpenPuppetWindowButton, PuppetSizeDecreaseButton, PuppetSizeIncreaseButton,
    PuppetSizeSlider,
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
            (
                @FeathersButton {
                    @caption: bsn! { Text("Open puppet window") }
                }
                OpenPuppetWindowButton
                InteractionDisabled
                on(puppet_window::open_puppet_window)
            ),
            Text("Puppet size"),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                }
                Children [
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("-") }
                        }
                        PuppetSizeDecreaseButton
                        on(puppet_window::decrease_puppet_size)
                    ),
                    (
                        @FeathersSlider {
                            @min: puppet_window::MIN_PUPPET_SIZE,
                            @max: puppet_window::MAX_PUPPET_SIZE,
                            @value: puppet_window::DEFAULT_PUPPET_SIZE,
                        }
                        PuppetSizeSlider
                        SliderStep(puppet_window::PUPPET_SIZE_STEP)
                        SliderPrecision(2)
                        on(puppet_window::set_puppet_size)
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("+") }
                        }
                        PuppetSizeIncreaseButton
                        on(puppet_window::increase_puppet_size)
                    ),
                ]
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { Text("Puppet window unlocked") }
                }
                Checked
                on(puppet_window::set_puppet_window_locked)
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { Text("Always on top") }
                }
                on(puppet_window::set_puppet_window_always_on_top)
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
            (Text("Level: -- dBFS") MicrophoneLevelText)
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
