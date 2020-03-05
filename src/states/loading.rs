use crate::audio::initialise_audio;
use crate::components::physics::PlatformCuboid;
use crate::level::Level;
use crate::states::pizzatopia::{MyEvents, Pizzatopia};
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

pub struct LoadingState {
    /// Tracks loaded assets.
    progress_counter: ProgressCounter,
}

impl Default for LoadingState {
    fn default() -> Self {
        LoadingState {
            progress_counter: ProgressCounter::default(),
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for LoadingState {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        let StateData { world, .. } = data;
        {
            initialise_audio(world);
        }
        let platform_size_prefab_handle = world.exec(|loader: PrefabLoader<'_, PlatformCuboid>| {
            loader.load("prefab/tile_size.ron", RonFormat, ())
        });
        world.insert(platform_size_prefab_handle.clone());

        let level_handle = world.read_resource::<Loader>().load(
            "levels/level0.ron", // Here we load the associated ron file
            RonFormat,
            &mut self.progress_counter,
            &world.read_resource::<AssetStorage<Level>>(),
        );
        world.insert(level_handle.clone());
        world.insert(Vec::<Handle<SpriteSheet>>::new());

        let tiles = load_spritesheet(String::from("texture/tiles"), world);
        let sprites = load_spritesheet(String::from("texture/spritesheet"), world);
        let sprites2 = load_spritesheet(String::from("texture/spritesheet2"), world);
        world
            .write_resource::<Vec<Handle<SpriteSheet>>>()
            .push(tiles);
        world
            .write_resource::<Vec<Handle<SpriteSheet>>>()
            .push(sprites);
        world
            .write_resource::<Vec<Handle<SpriteSheet>>>()
            .push(sprites2);
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        if self.progress_counter.is_complete() {
            Trans::Switch(Box::new(Pizzatopia::default()))
        } else {
            Trans::None
        }
    }
}

fn load_spritesheet(filename_without_extension: String, world: &mut World) -> Handle<SpriteSheet> {
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
        (),
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
        sampler_info: SamplerInfo::new(Filter::Linear, WrapMode::Clamp),
        // Don't generate mipmaps for this image
        generate_mips: false,
        premultiply_alpha: true,
    }
}
