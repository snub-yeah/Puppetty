use bevy::camera::{ClearColorConfig, RenderTarget, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use bevy::window::CompositeAlphaMode;
use bevy::window::{
    CursorOptions, PrimaryWindow, WindowCloseRequested, WindowClosed, WindowLevel, WindowRef,
};

use crate::image_selection::SelectedImage;
use crate::microphone::{
    MAX_MICROPHONE_LEVEL_DBFS, MIN_MICROPHONE_LEVEL_DBFS, MicrophoneLevel, microphone_level_dbfs,
};

const PUPPET_RENDER_LAYER: usize = 1;
pub(crate) const MIN_PUPPET_SIZE: f32 = 0.25;
pub(crate) const MAX_PUPPET_SIZE: f32 = 3.0;
pub(crate) const DEFAULT_PUPPET_SIZE: f32 = 1.0;
pub(crate) const PUPPET_SIZE_STEP: f32 = 0.05;
pub(crate) const MIN_PUPPET_Y: f32 = -500.0;
pub(crate) const MAX_PUPPET_Y: f32 = 500.0;
pub(crate) const DEFAULT_MIN_PUPPET_Y: f32 = -200.0;
pub(crate) const DEFAULT_MAX_PUPPET_Y: f32 = 200.0;
pub(crate) const DEFAULT_MINIMUM_INPUT_LEVEL_DBFS: f32 = -45.0;
pub(crate) const DEFAULT_MAXIMUM_INPUT_LEVEL_DBFS: f32 = -15.0;
const PUPPET_FALL_SPEED: f32 = 600.0;
const MAX_PUPPET_RISE_SPEED: f32 = 600.0;
const LOUDNESS_CURVE: f32 = 2.0;

pub(crate) struct PuppetWindowPlugin;

impl Plugin for PuppetWindowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuppetWindowState>()
            .init_resource::<TransformInfo>()
            .add_systems(
                Update,
                (
                    enable_open_puppet_window_button,
                    close_puppet_window_when_config_closes,
                    cleanup_closed_puppet_window,
                    apply_puppet_window_settings,
                    calculate_transform_based_on_mic_volume,
                    move_puppet_sprite,
                    apply_puppet_sprite_transform,
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

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetMinYSlider;

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetMaxYSlider;

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetMinimumInputLevelSlider;

#[derive(Component, Clone, Default)]
pub(crate) struct PuppetMaximumInputLevelSlider;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct PuppetSprite {
    position: Vec2,
    rotation: f32,
}

#[derive(Resource, Default)]
pub(crate) struct TransformInfo {
    target_y: f32,
    rise_speed: f32,
}

#[derive(Resource)]
pub(crate) struct PuppetWindowState {
    window: Option<Entity>,
    camera: Option<Entity>,
    sprite: Option<Entity>,
    size: f32,
    locked: bool,
    always_on_top: bool,
    min_y: f32,
    max_y: f32,
    minimum_input_level_dbfs: f32,
    maximum_input_level_dbfs: f32,
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
            min_y: DEFAULT_MIN_PUPPET_Y,
            max_y: DEFAULT_MAX_PUPPET_Y,
            minimum_input_level_dbfs: DEFAULT_MINIMUM_INPUT_LEVEL_DBFS,
            maximum_input_level_dbfs: DEFAULT_MAXIMUM_INPUT_LEVEL_DBFS,
        }
    }
}

fn apply_puppet_sprite_transform(
    mut sprites: Query<(&PuppetSprite, &mut Transform), Changed<PuppetSprite>>,
) {
    for (puppet_sprite, mut transform) in &mut sprites {
        transform.translation = puppet_sprite.position.extend(transform.translation.z);
        transform.rotation = Quat::from_rotation_z(puppet_sprite.rotation);
    }
}

fn move_puppet_sprite(
    transform_info: Res<TransformInfo>,
    time: Res<Time>,
    mut sprites: Query<&mut PuppetSprite>,
) {
    for mut sprite in &mut sprites {
        let speed = if sprite.position.y < transform_info.target_y {
            transform_info.rise_speed
        } else {
            PUPPET_FALL_SPEED
        };
        let maximum_distance = speed * time.delta_secs();
        let distance_to_target = transform_info.target_y - sprite.position.y;
        sprite.position.y += distance_to_target.clamp(-maximum_distance, maximum_distance);
    }
}

fn calculate_transform_based_on_mic_volume(
    volume: Res<MicrophoneLevel>,
    state: Res<PuppetWindowState>,
    mut transform_info: ResMut<TransformInfo>,
) {
    let rms = f32::from_bits(volume.value.load(std::sync::atomic::Ordering::Relaxed));
    let transform_ratio = puppet_movement_for_input_level(
        microphone_level_dbfs(rms),
        state.minimum_input_level_dbfs,
        state.maximum_input_level_dbfs,
    );
    transform_info.target_y = state.min_y.lerp(state.max_y, transform_ratio);
    transform_info.rise_speed = transform_ratio * MAX_PUPPET_RISE_SPEED;
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
            PuppetSprite::default(),
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

pub(crate) fn set_puppet_min_y(
    change: On<ValueChange<f32>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.min_y = clamp_puppet_y(change.value).min(state.max_y);
    commands
        .entity(change.source)
        .insert(SliderValue(state.min_y));
}

pub(crate) fn set_puppet_max_y(
    change: On<ValueChange<f32>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.max_y = clamp_puppet_y(change.value).max(state.min_y);
    commands
        .entity(change.source)
        .insert(SliderValue(state.max_y));
}

pub(crate) fn set_puppet_minimum_input_level(
    change: On<ValueChange<f32>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.minimum_input_level_dbfs =
        clamp_input_level(change.value).min(state.maximum_input_level_dbfs - 1.0);
    commands
        .entity(change.source)
        .insert(SliderValue(state.minimum_input_level_dbfs));
}

pub(crate) fn set_puppet_maximum_input_level(
    change: On<ValueChange<f32>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    state.maximum_input_level_dbfs =
        clamp_input_level(change.value).max(state.minimum_input_level_dbfs + 1.0);
    commands
        .entity(change.source)
        .insert(SliderValue(state.maximum_input_level_dbfs));
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
        if state.window == Some(closed_window.window) {
            cleanup_puppet_window(&mut state, &mut commands);
        }
    }
}

fn close_puppet_window_when_config_closes(
    mut close_requests: MessageReader<WindowCloseRequested>,
    config_windows: Query<(), With<PrimaryWindow>>,
    mut state: ResMut<PuppetWindowState>,
    mut commands: Commands,
) {
    for close_request in close_requests.read() {
        if config_windows.get(close_request.window).is_ok() {
            close_puppet_window(&mut state, &mut commands);
        }
    }
}

fn close_puppet_window(state: &mut PuppetWindowState, commands: &mut Commands) {
    if let Some(window) = state.window {
        commands.entity(window).try_despawn();
    }
    cleanup_puppet_window(state, commands);
}

fn cleanup_puppet_window(state: &mut PuppetWindowState, commands: &mut Commands) {
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

fn clamp_puppet_size(size: f32) -> f32 {
    size.clamp(MIN_PUPPET_SIZE, MAX_PUPPET_SIZE)
}

fn clamp_puppet_y(y: f32) -> f32 {
    y.clamp(MIN_PUPPET_Y, MAX_PUPPET_Y)
}

fn clamp_input_level(level: f32) -> f32 {
    level.clamp(MIN_MICROPHONE_LEVEL_DBFS, MAX_MICROPHONE_LEVEL_DBFS)
}

fn puppet_movement_for_input_level(
    input_level_dbfs: f32,
    minimum_input_level_dbfs: f32,
    maximum_input_level_dbfs: f32,
) -> f32 {
    get_puppet_transform_ratio(
        input_level_dbfs,
        minimum_input_level_dbfs,
        maximum_input_level_dbfs,
    )
    .powf(LOUDNESS_CURVE)
}

fn get_puppet_transform_ratio(
    input_level_dbfs: f32,
    minimum_input_level_dbfs: f32,
    maximum_input_level_dbfs: f32,
) -> f32 {
    if input_level_dbfs <= minimum_input_level_dbfs {
        return 0.;
    }

    let input_range = maximum_input_level_dbfs - minimum_input_level_dbfs;
    let amount = ((input_level_dbfs - minimum_input_level_dbfs) / input_range).clamp(0.0, 1.0);
    amount
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

    #[test]
    fn puppet_y_stays_within_supported_range() {
        assert_eq!(clamp_puppet_y(-600.0), MIN_PUPPET_Y);
        assert_eq!(clamp_puppet_y(600.0), MAX_PUPPET_Y);
        assert_eq!(clamp_puppet_y(0.0), 0.0);
    }

    #[test]
    fn input_level_stays_within_meter_range() {
        assert_eq!(clamp_input_level(-100.0), -80.0);
        assert_eq!(clamp_input_level(10.0), 0.0);
    }
}
