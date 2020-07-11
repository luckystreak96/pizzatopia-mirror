use crate::{
    audio::initialise_audio,
    components::{
        graphics::{SpriteSheetType, SPRITESHEETTYPE_COUNT},
        physics::PlatformCuboid,
    },
    level::Level,
    states::pizzatopia::{MyEvents, Pizzatopia},
    systems::input::InputManager,
    ui::{
        file_picker::{FilePickerFilename, DIR_LEVELS},
        UiStack,
    },
};
use amethyst::{
    assets::{
        Asset, AssetStorage, Completion, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, Progress, ProgressCounter, RonFormat,
    },
    core::{
        bundle::SystemBundle,
        ecs::{Read, SystemData, World},
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        transform::Transform,
        EventReader, SystemDesc, Time,
    },
    derive::EventReader,
    ecs::prelude::{Component, DenseVecStorage, Dispatcher, DispatcherBuilder, Entity},
    input::{is_key_down, InputHandler, StringBindings, VirtualKeyCode},
    prelude::*,
    renderer::{
        debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams},
        rendy::{
            hal::image::{Filter, SamplerInfo, WrapMode},
            texture::image::{ImageTextureConfig, Repr, TextureKind},
        },
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
    },
    ui::{FontAsset, RenderUi, TtfFormat, UiBundle, UiCreator, UiEvent, UiFinder, UiText},
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    winit::Event,
};
use log::error;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct LoadLevelState {
    /// Tracks loaded assets.
    level_progress: ProgressCounter,
    level_handle: Option<Handle<Level>>,
}

impl Default for LoadLevelState {
    fn default() -> Self {
        LoadLevelState {
            level_progress: ProgressCounter::default(),
            level_handle: None,
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for LoadLevelState {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        let mut path = PathBuf::new();
        path.push(DIR_LEVELS);
        let filename = data
            .world
            .read_resource::<FilePickerFilename>()
            .filename
            .clone();
        path = path.join(filename.as_str());
        self.level_handle = Some(data.world.read_resource::<Loader>().load(
            path.to_str().unwrap(), // Here we load the associated ron file
            RonFormat,
            &mut self.level_progress,
            &data.world.read_resource::<AssetStorage<Level>>(),
        ));
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        match self.level_progress.complete() {
            Completion::Failed => {
                error!("Failed to load Level asset");
                Trans::Switch(Box::new(Pizzatopia::default()))
            }
            Completion::Complete => {
                data.world
                    .insert(self.level_handle.clone().unwrap().clone());
                Trans::Switch(Box::new(Pizzatopia::default()))
            }
            Completion::Loading => Trans::None,
        }
    }
}
