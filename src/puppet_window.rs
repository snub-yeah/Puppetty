use bevy::camera::{ClearColorConfig, RenderTarget, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use bevy::window::CompositeAlphaMode;
use bevy::window::{CursorOptions, WindowClosed, WindowLevel, WindowRef};

use crate::image_selection::SelectedImage;

//TODO: on config window close, close this window too

const PUPPET_RENDER_LAYER: usize = 1;
pub(crate) const MIN_PUPPET_SIZE: f32 = 0.25;
pub(crate) const MAX_PUPPET_SIZE: f32 = 3.0;
pub(crate) const DEFAULT_PUPPET_SIZE: f32 = 1.0;
pub(crate) const PUPPET_SIZE_STEP: f32 = 0.05;

pub(crate) struct PuppetWindowPlugin;

impl Plugin for PuppetWindowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuppetWindowState>().add_systems(
            Update,
            (
                enable_open_puppet_window_button,
                cleanup_closed_puppet_window,
                apply_puppet_window_settings,
            )
                .chain(),
        );
    }
}

#[derive(Component, Clone, Default)]
pub(crate) struct OpenPuppetWindowButton;

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetSizeDecreaseButton;

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetSizeIncreaseButton;

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetSizeSlider;

#[derive(Resource)]
pub(crate) struct PuppetWindowState {
    window: Option<Entity>,
    camera: Option<Entity>,
    sprite: Option<Entity>,
    size: f32,
    locked: bool,
    always_on_top: bool,
}

impl Default for PuppetWindowState {
    fn default() -> Self {
        Self {
            window: None,
            camera: None,
            sprite: None,
            size: DEFAULT_PUPPET_SIZE,
            locked: false,
            always_on_top: false,
        }
    }
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
        commands.entity(sprite).insert((
            Sprite::from_image(image),
            Transform::from_scale(Vec3::splat(state.size)),
        ));
        return;
    }

    let window = commands
        .spawn((
            Window {
                title: "Puppetty".to_string(),
                transparent: true,
                decorations: !state.locked,
                window_level: window_level(state.always_on_top),
                #[cfg(target_os = "macos")]
                composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                #[cfg(target_os = "linux")]
                composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                ..default()
            },
            CursorOptions {
                hit_test: !state.locked,
                ..default()
            },
        ))
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
            Transform::from_scale(Vec3::splat(state.size)),
            RenderLayers::layer(PUPPET_RENDER_LAYER),
        ))
        .id();

    state.window = Some(window);
    state.camera = Some(camera);
    state.sprite = Some(sprite);
}

pub(crate) fn decrease_puppet_size(
    _activate: On<Activate>,
    mut state: ResMut<PuppetWindowState>,
    sliders: Query<Entity, With<PuppetSizeSlider>>,
    mut commands: Commands,
) {
    state.size = clamp_puppet_size(state.size - PUPPET_SIZE_STEP);
    for slider in sliders {
        commands.entity(slider).insert(SliderValue(state.size));
    }
}

pub(crate) fn increase_puppet_size(
    _activate: On<Activate>,
    mut state: ResMut<PuppetWindowState>,
    sliders: Query<Entity, With<PuppetSizeSlider>>,
    mut commands: Commands,
) {
    state.size = clamp_puppet_size(state.size + PUPPET_SIZE_STEP);
    for slider in sliders {
        commands.entity(slider).insert(SliderValue(state.size));
    }
}

pub(crate) fn set_puppet_size(
    change: On<ValueChange<f32>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.size = clamp_puppet_size(change.value);
    commands
        .entity(change.source)
        .insert(SliderValue(state.size));
}

pub(crate) fn set_puppet_window_locked(
    change: On<ValueChange<bool>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.locked = !change.value;
    let mut checkbox = commands.entity(change.source);
    if change.value {
        checkbox.insert(Checked);
    } else {
        checkbox.remove::<Checked>();
    }
}

pub(crate) fn set_puppet_window_always_on_top(
    change: On<ValueChange<bool>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.always_on_top = change.value;
    let mut checkbox = commands.entity(change.source);
    if change.value {
        checkbox.insert(Checked);
    } else {
        checkbox.remove::<Checked>();
    }
}

fn apply_puppet_window_settings(
    state: Res<PuppetWindowState>,
    mut windows: Query<(&mut Window, &mut CursorOptions)>,
    mut sprites: Query<&mut Transform>,
) {
    if !state.is_changed() {
        return;
    }

    if let Some(window) = state.window
        && let Ok((mut window_settings, mut cursor_options)) = windows.get_mut(window)
    {
        window_settings.decorations = !state.locked;
        window_settings.window_level = window_level(state.always_on_top);
        cursor_options.hit_test = !state.locked;
    }
    if let Some(sprite) = state.sprite
        && let Ok(mut transform) = sprites.get_mut(sprite)
    {
        transform.scale = Vec3::splat(state.size);
    }
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
        state.window = None;
        state.camera = None;
        state.sprite = None;
    }
}

fn clamp_puppet_size(size: f32) -> f32 {
    size.clamp(MIN_PUPPET_SIZE, MAX_PUPPET_SIZE)
}

fn window_level(always_on_top: bool) -> WindowLevel {
    if always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puppet_size_stays_within_supported_range() {
        assert_eq!(clamp_puppet_size(0.0), MIN_PUPPET_SIZE);
        assert_eq!(clamp_puppet_size(4.0), MAX_PUPPET_SIZE);
        assert_eq!(clamp_puppet_size(DEFAULT_PUPPET_SIZE), DEFAULT_PUPPET_SIZE);
    }
}
