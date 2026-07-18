use bevy::asset::UnapprovedPathMode;
use bevy::feathers::controls::FeathersButton;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::{ThemeBackgroundColor, UiTheme};
use bevy::feathers::*;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy::ui_widgets::Activate;
use std::path::PathBuf;

#[derive(Component, Clone, Default)]
struct SelectedImagePath {
    path: Option<PathBuf>,
}

#[derive(Component, Clone, Default)]
struct PreviewImage;

#[derive(Component)]
struct FileDialogTask {
    task: Task<Option<rfd::FileHandle>>,
}

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
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, poll_file_dialog)
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, config_window()]
}

fn config_window() -> impl Scene {
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
                on(
                    |_activate: On<Activate>,
                     mut commands: Commands,
                     dialogs: Query<(), With<FileDialogTask>>| {
                        if dialogs.is_empty() {
                            commands.spawn(FileDialogTask {
                                task: AsyncComputeTaskPool::get().spawn(
                                    rfd::AsyncFileDialog::new()
                                        .add_filter(
                                            "Image",
                                            &["png"]//for now just png cuz others arent working idk why
                                        )
                                        .pick_file(),
                                ),
                            });
                        }
                    }
                )
            ),
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
            .override_unapproved() //allow unapproved cuz the user is selecting the images here, its up to them to not select sumn crazy lol
            .load(file.path().to_string_lossy().to_string());
        selected_image_path.path = Some(file.path().to_path_buf());
        image_node.image = handle;
        *visibility = Visibility::Inherited;
    }
}
