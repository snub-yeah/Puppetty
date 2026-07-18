use bevy::prelude::*;
use cpal::Sample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

pub(crate) struct MicrophonePlugin;

impl Plugin for MicrophonePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(discover_microphones())
            .init_resource::<MicrophoneCapture>()
            .init_resource::<MicrophoneLevel>()
            .init_resource::<MicrophoneError>()
            .add_systems(Startup, start_default_microphone)
            .add_systems(Update, update_microphone_level);
    }
}

#[derive(Component, Clone, Default)]
pub(crate) struct MicrophoneOption {
    pub(crate) index: usize,
}

#[derive(Component, Clone, Default)]
pub(crate) struct CurrentMicrophoneText;

#[derive(Component, Clone, Default)]
pub(crate) struct MicrophoneLevelText;

#[derive(Resource)]
pub(crate) struct MicrophoneDevices {
    pub(crate) devices: Vec<MicrophoneDevice>,
}

pub(crate) struct MicrophoneDevice {
    pub(crate) name: String,
    devices: Vec<cpal::Device>,
    is_physical: bool,
}

#[derive(Resource, Default)]
pub(crate) struct MicrophoneCapture {
    stream: Option<cpal::Stream>,
}

#[derive(Resource, Clone)]
pub(crate) struct MicrophoneLevel {
    value: Arc<AtomicU32>,
}

impl Default for MicrophoneLevel {
    fn default() -> Self {
        Self {
            value: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct MicrophoneError {
    message: Option<String>,
}

fn discover_microphones() -> MicrophoneDevices {
    let host = cpal::default_host();
    let default_device = host.default_input_device();
    let default_id = default_device.as_ref().and_then(device_id);
    let mut microphones = Vec::new();

    if let Some(device) = default_device {
        microphones.push(MicrophoneDevice {
            name: "System default".to_string(),
            devices: vec![device],
            is_physical: false,
        });
    }

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if device_id(&device).as_ref() == default_id.as_ref() {
                continue;
            }

            let name = device_name(&device).unwrap_or_else(|| "Input device".to_string());
            let is_physical = device_id(&device).is_some_and(|id| id.starts_with("plughw:"));

            if let Some(microphone) = microphones
                .iter_mut()
                .find(|microphone| microphone.name == name)
            {
                microphone.devices.push(device);
                microphone.is_physical |= is_physical;
            } else {
                microphones.push(MicrophoneDevice {
                    name,
                    devices: vec![device],
                    is_physical,
                });
            }
        }
    }

    #[cfg(target_os = "linux")]
    microphones.retain(|microphone| microphone.name == "System default" || microphone.is_physical);

    MicrophoneDevices {
        devices: microphones,
    }
}

fn device_name(device: &cpal::Device) -> Option<String> {
    device
        .description()
        .ok()
        .map(|description| description.name().to_string())
}

fn device_id(device: &cpal::Device) -> Option<String> {
    device.id().ok().map(|id| id.1)
}

fn start_default_microphone(
    devices: Res<MicrophoneDevices>,
    mut capture: ResMut<MicrophoneCapture>,
    level: Res<MicrophoneLevel>,
    mut error: ResMut<MicrophoneError>,
) {
    if devices.devices.is_empty() {
        error.message = Some("No default input device is available".to_string());
        return;
    }

    select_microphone_by_index(0, &devices, &mut capture, &level, &mut error);
}

pub(crate) fn select_microphone(
    activate: On<bevy::ui_widgets::Activate>,
    options: Query<&MicrophoneOption>,
    devices: Res<MicrophoneDevices>,
    mut capture: ResMut<MicrophoneCapture>,
    level: Res<MicrophoneLevel>,
    mut error: ResMut<MicrophoneError>,
    mut current_microphone: Query<&mut Text, With<CurrentMicrophoneText>>,
) {
    let Ok(option) = options.get(activate.entity) else {
        return;
    };

    if select_microphone_by_index(option.index, &devices, &mut capture, &level, &mut error) {
        current_microphone.single_mut().unwrap().0 = devices.devices[option.index].name.clone();
    }
}

fn select_microphone_by_index(
    index: usize,
    devices: &MicrophoneDevices,
    capture: &mut MicrophoneCapture,
    level: &MicrophoneLevel,
    error: &mut MicrophoneError,
) -> bool {
    let Some(microphone) = devices.devices.get(index) else {
        error.message = Some("The selected microphone is no longer available".to_string());
        return false;
    };

    let mut last_error = None;
    for device in &microphone.devices {
        let config = match device.default_input_config() {
            Ok(config) => config,
            Err(stream_error) => {
                last_error = Some(stream_error.to_string());
                continue;
            }
        };
        let stream_config = config.clone().into();
        let stream = match build_level_stream(
            device,
            &stream_config,
            config.sample_format(),
            &level.value,
        ) {
            Ok(stream) => stream,
            Err(stream_error) => {
                last_error = Some(stream_error);
                continue;
            }
        };
        if let Err(stream_error) = stream.play() {
            last_error = Some(stream_error.to_string());
            continue;
        }

        level.value.store(0.0_f32.to_bits(), Ordering::Relaxed);
        capture.stream = Some(stream);
        error.message = None;
        return true;
    }

    let detail = last_error.unwrap_or_else(|| "no compatible input configuration".to_string());
    error.message = Some(format!("Could not start {}: {detail}", microphone.name));
    false
}

fn build_level_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    level: &Arc<AtomicU32>,
) -> Result<cpal::Stream, String> {
    macro_rules! stream_for_format {
        ($sample:ty) => {
            build_level_stream_for_samples::<$sample>(device, config, Arc::clone(level))
        };
    }

    let stream = match sample_format {
        cpal::SampleFormat::I8 => stream_for_format!(i8),
        cpal::SampleFormat::I16 => stream_for_format!(i16),
        cpal::SampleFormat::I24 => stream_for_format!(cpal::I24),
        cpal::SampleFormat::I32 => stream_for_format!(i32),
        cpal::SampleFormat::I64 => stream_for_format!(i64),
        cpal::SampleFormat::U8 => stream_for_format!(u8),
        cpal::SampleFormat::U16 => stream_for_format!(u16),
        cpal::SampleFormat::U24 => stream_for_format!(cpal::U24),
        cpal::SampleFormat::U32 => stream_for_format!(u32),
        cpal::SampleFormat::U64 => stream_for_format!(u64),
        cpal::SampleFormat::F32 => stream_for_format!(f32),
        cpal::SampleFormat::F64 => stream_for_format!(f64),
        _ => {
            return Err(format!(
                "Unsupported microphone sample format: {sample_format}"
            ));
        }
    };

    stream.map_err(|stream_error| stream_error.to_string())
}

fn build_level_stream_for_samples<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    level: Arc<AtomicU32>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    device.build_input_stream(
        config,
        move |samples: &[T], _| update_microphone_rms(samples, &level),
        |stream_error| bevy::log::error!("Microphone input stream error: {stream_error}"),
        None,
    )
}

fn update_microphone_rms<T>(samples: &[T], level: &AtomicU32)
where
    T: cpal::Sample,
    f32: cpal::FromSample<T>,
{
    if samples.is_empty() {
        return;
    }

    let sum_of_squares = samples.iter().fold(0.0, |sum, sample| {
        let sample = f32::from_sample(*sample);
        sum + sample * sample
    });
    let rms = (sum_of_squares / samples.len() as f32).sqrt();
    level.store(rms.to_bits(), Ordering::Relaxed);
}

fn update_microphone_level(
    level: Res<MicrophoneLevel>,
    capture: Res<MicrophoneCapture>,
    error: Res<MicrophoneError>,
    mut level_text: Single<&mut Text, With<MicrophoneLevelText>>,
) {
    let text = if let Some(error) = &error.message {
        format!("Level: {error}")
    } else if capture.stream.is_some() {
        let rms = f32::from_bits(level.value.load(Ordering::Relaxed));
        let decibels = 20.0 * rms.max(0.000_01).log10();
        format!("Level: {decibels:.1} dBFS")
    } else {
        "Level: unavailable".to_string()
    };

    if level_text.0 != text {
        level_text.0 = text;
    }
}
