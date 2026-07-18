mod config;
mod image_selection;
mod microphone;
mod puppet_window;

use bevy::asset::UnapprovedPathMode;
use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;
use config::ConfigPlugin;
use image_selection::ImageSelectionPlugin;
use microphone::MicrophonePlugin;
use puppet_window::PuppetWindowPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                unapproved_path_mode: UnapprovedPathMode::Deny,
                ..default()
            }),
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_plugins((
            MicrophonePlugin,
            ImageSelectionPlugin,
            PuppetWindowPlugin,
            ConfigPlugin,
        ))
        .run();
}
