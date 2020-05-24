use crate::audio::initialise_audio;
use crate::components::graphics::SpriteSheetType;
use crate::components::physics::PlatformCuboid;
use crate::level::Level;
use crate::states::pizzatopia::{MyEvents, Pizzatopia};
use crate::systems::input::InputManager;
use crate::ui::file_picker::{FilePickerFilename, DIR_LEVELS};
use crate::ui::UiStack;
use amethyst::assets::Completion;
use amethyst::assets::Progress;
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, ProgressCounter, RonFormat, Source,
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
    ecs::prelude::{Component, DenseVecStorage, Dispatcher, DispatcherBuilder, Entity, Join},
    input::{is_key_down, InputHandler, StringBindings, VirtualKeyCode},
    prelude::*,
    renderer::{
        rendy::{
            hal::image::{Filter, SamplerInfo, WrapMode},
            texture::image::{ImageTextureConfig, Repr, TextureKind},
        },
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
    },
    ui::{RenderUi, UiBundle, UiCreator, UiEvent, UiFinder, UiText},
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    winit::Event,
};
use log::error;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct AssetsDir(pub PathBuf);

pub struct LoadingState {
    /// Tracks loaded assets.
    progress_counter: ProgressCounter,
    level_progress: ProgressCounter,
}

impl Default for LoadingState {
    fn default() -> Self {
        LoadingState {
            progress_counter: ProgressCounter::default(),
            level_progress: ProgressCounter::default(),
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for LoadingState {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        initialise_audio(data.world);
        let platform_size_prefab_handle =
            data.world.exec(|loader: PrefabLoader<'_, PlatformCuboid>| {
                loader.load("prefab/tile_size.ron", RonFormat, ())
            });
        data.world.insert(platform_size_prefab_handle.clone());

        data.world
            .insert(AssetsDir(application_root_dir().unwrap().join("assets")));
        let filename = "level0.ron";
        let path = PathBuf::from(DIR_LEVELS)
            .join(filename)
            .display()
            .to_string();
        let level_handle = data.world.read_resource::<Loader>().load(
            path.as_str(), // Here we load the associated ron file
            RonFormat,
            &mut self.level_progress,
            &data.world.read_resource::<AssetStorage<Level>>(),
        );
        data.world.insert(level_handle.clone());
        data.world
            .insert(BTreeMap::<u8, Handle<SpriteSheet>>::new());

        let name = String::from("texture/tiles");
        let tiles = load_spritesheet(name.clone(), data.world, &mut self.progress_counter);
        data.world
            .write_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            .insert(SpriteSheetType::Tiles as u8, tiles);

        let name = String::from("texture/spritesheet");
        let sprites = load_spritesheet(name.clone(), data.world, &mut self.progress_counter);
        data.world
            .write_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            .insert(SpriteSheetType::Didi as u8, sprites);

        let name = String::from("texture/spritesheet2");
        let sprites2 = load_spritesheet(name.clone(), data.world, &mut self.progress_counter);
        data.world
            .write_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            .insert(SpriteSheetType::Snap as u8, sprites2);

        data.world.insert(InputManager::new(data.world));
        data.world.insert(FilePickerFilename::new(
            "insert filename here".to_string(),
            "insert fullpath here".to_string(),
        ));
        data.world.insert(UiStack::default());
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        if self.progress_counter.is_complete() {
            match self.level_progress.complete() {
                Completion::Failed => {
                    error!("Failed to load Level asset");
                    Trans::Switch(Box::new(Pizzatopia::default()))
                }
                Completion::Complete => Trans::Switch(Box::new(Pizzatopia::default())),
                _ => Trans::None,
            }
        } else {
            Trans::None
        }
    }
}

fn load_spritesheet(
    filename_without_extension: String,
    world: &mut World,
    progress: &mut ProgressCounter,
) -> Handle<SpriteSheet> {
    // Load the sprite sheet necessary to render the graphics.
    // The texture is the pixel data
    // `texture_handle` is a cloneable reference to the texture
    let texture_handle = {
        let loader = world.read_resource::<Loader>();
        let texture_storage = world.read_resource::<AssetStorage<Texture>>();
        loader.load(
            filename_without_extension.clone() + ".png",
            ImageFormat(get_image_texure_config()),
            (),
            &texture_storage,
        )
    };

    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
        filename_without_extension.clone() + ".ron", // Here we load the associated ron file
        SpriteSheetFormat(texture_handle),
        progress,
        &sprite_sheet_store,
    )
}

fn get_image_texure_config() -> ImageTextureConfig {
    ImageTextureConfig {
        // Determine format automatically
        format: None,
        // Color channel
        repr: Repr::Srgb,
        // Two-dimensional texture
        kind: TextureKind::D2,
        sampler_info: SamplerInfo::new(Filter::Nearest, WrapMode::Clamp),
        // Don't generate mipmaps for this image
        generate_mips: false,
        premultiply_alpha: true,
    }
}
