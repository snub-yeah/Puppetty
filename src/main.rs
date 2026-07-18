mod config;
mod image_selection;
mod microphone;

use bevy::asset::UnapprovedPathMode;
use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;
use config::ConfigPlugin;
use image_selection::ImageSelectionPlugin;
use microphone::MicrophonePlugin;

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
        .add_plugins((MicrophonePlugin, ImageSelectionPlugin, ConfigPlugin))
        .run();
}
