use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::ui_widgets::Activate;
use std::path::PathBuf;

pub(crate) struct ImageSelectionPlugin;

impl Plugin for ImageSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, poll_file_dialog);
    }
}

#[derive(Component, Clone, Default)]
pub(crate) struct SelectedImagePath {
    path: Option<PathBuf>,
}

#[derive(Component, Clone, Default)]
pub(crate) struct PreviewImage;

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
    preview: Single<(&mut SelectedImagePath, &mut ImageNode, &mut Visibility), With<PreviewImage>>,
    asset_server: Res<AssetServer>,
) {
    let (mut selected_image_path, mut image_node, mut visibility) = preview.into_inner();

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
        selected_image_path.path = Some(file.path().to_path_buf());
        image_node.image = handle;
        *visibility = Visibility::Inherited;
    }
}
