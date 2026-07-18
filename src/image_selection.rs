use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::ui_widgets::Activate;

pub(crate) struct ImageSelectionPlugin;

impl Plugin for ImageSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedImage>()
            .add_systems(Update, poll_file_dialog);
    }
}

#[derive(Resource, Default)]
pub(crate) struct SelectedImage {
    pub(crate) image: Option<Handle<Image>>,
}

#[derive(Component)]
pub(crate) struct FileDialogTask {
    task: Task<Option<rfd::FileHandle>>,
}

pub(crate) fn begin_file_selection(
    _activate: On<Activate>,
    mut commands: Commands,
    dialogs: Query<(), With<FileDialogTask>>,
) {
    if dialogs.is_empty() {
        commands.spawn(FileDialogTask {
            task: AsyncComputeTaskPool::get().spawn(
                rfd::AsyncFileDialog::new()
                    .add_filter("Image", &["png"])
                    .pick_file(),
            ),
        });
    }
}

fn poll_file_dialog(
    mut commands: Commands,
    mut dialogs: Query<(Entity, &mut FileDialogTask)>,
    mut selected_image: ResMut<SelectedImage>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut dialog) in &mut dialogs {
        let Some(result) = block_on(poll_once(&mut dialog.task)) else {
            continue;
        };

        commands.entity(entity).despawn();

        let Some(file) = result else {
            continue;
        };
        let handle: Handle<Image> = asset_server
            .load_builder()
            .override_unapproved()
            .load(file.path().to_string_lossy().to_string());
        selected_image.image = Some(handle);
    }
}
