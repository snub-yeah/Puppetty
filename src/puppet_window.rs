use bevy::camera::{ClearColorConfig, RenderTarget, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use bevy::window::CompositeAlphaMode;
use bevy::window::{WindowClosed, WindowRef};

use crate::image_selection::SelectedImage;

const PUPPET_RENDER_LAYER: usize = 1;

pub(crate) struct PuppetWindowPlugin;

impl Plugin for PuppetWindowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuppetWindowState>().add_systems(
            Update,
            (
                enable_open_puppet_window_button,
                cleanup_closed_puppet_window,
            )
                .chain(),
        );
    }
}

#[derive(Component, Clone, Default)]
pub(crate) struct OpenPuppetWindowButton;

#[derive(Resource, Default)]
pub(crate) struct PuppetWindowState {
    window: Option<Entity>,
    camera: Option<Entity>,
    sprite: Option<Entity>,
}

fn enable_open_puppet_window_button(
    selected_image: Res<SelectedImage>,
    buttons: Query<Entity, (With<OpenPuppetWindowButton>, With<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if !selected_image.is_changed() || selected_image.image.is_none() {
        return;
    }

    for button in buttons {
        commands.entity(button).remove::<InteractionDisabled>();
    }
}

pub(crate) fn open_puppet_window(
    _activate: On<Activate>,
    selected_image: Res<SelectedImage>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    let Some(image) = selected_image.image.clone() else {
        return;
    };

    if let Some(sprite) = state.sprite {
        commands.entity(sprite).insert(Sprite::from_image(image));
        return;
    }

    let window = commands
        .spawn(Window {
            title: "Puppetty".to_string(),
            transparent: true,
            #[cfg(target_os = "macos")]
            composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
            #[cfg(target_os = "linux")]
            composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
            ..default()
        })
        .id();
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            RenderLayers::layer(PUPPET_RENDER_LAYER),
            RenderTarget::Window(WindowRef::Entity(window)),
        ))
        .id();
    let sprite = commands
        .spawn((
            Sprite::from_image(image),
            RenderLayers::layer(PUPPET_RENDER_LAYER),
        ))
        .id();

    state.window = Some(window);
    state.camera = Some(camera);
    state.sprite = Some(sprite);
}

fn cleanup_closed_puppet_window(
    mut closed_windows: MessageReader<WindowClosed>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    for closed_window in closed_windows.read() {
        if state.window != Some(closed_window.window) {
            continue;
        }

        if let Some(camera) = state.camera {
            commands.entity(camera).try_despawn();
        }
        if let Some(sprite) = state.sprite {
            commands.entity(sprite).try_despawn();
        }
        *state = default();
    }
}
