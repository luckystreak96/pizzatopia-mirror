use crate::components::physics::{
    Collidee, Grounded, PlatformCollisionPoints, PlatformCuboid, Position, Velocity,
};
use crate::components::player::Player;
use crate::level::Level;
use crate::utils::Vec2;
use amethyst::input::{InputHandler, StringBindings};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, ProgressCounter, RonFormat, Source,
    },
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use amethyst::renderer::rendy::hal::image::{Filter, SamplerInfo, WrapMode};
use amethyst::renderer::rendy::texture::image::{ImageTextureConfig, Repr, TextureKind};
use log::info;

pub const CAM_HEIGHT: f32 = TILE_HEIGHT * 12.0;
pub const CAM_WIDTH: f32 = TILE_WIDTH * 16.0;

pub const TILE_WIDTH: f32 = 128.0;
pub const TILE_HEIGHT: f32 = 128.0;

pub const MAX_FALL_SPEED: f32 = 5.0;
pub const MAX_RUN_SPEED: f32 = 5.0;

pub(crate) struct Pizzatopia {
    pub level_handle: Handle<Level>,
}

impl SimpleState for Pizzatopia {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let world = data.world;

        world.register::<PlatformCuboid>();
        world.register::<PlatformCollisionPoints>();

        let sprite_sheet_handle = load_sprite_sheet(world);
        let prefab_handle = world.exec(|loader: PrefabLoader<'_, PlatformCuboid>| {
            loader.load("prefab/tile_size.ron", RonFormat, ())
        });

        initialise_actor(
            Vec2::new(CAM_WIDTH / 2.0, CAM_HEIGHT / 2.0),
            true,
            world,
            sprite_sheet_handle.clone(),
        );
        initialise_actor(
            Vec2::new(CAM_WIDTH / 2.0 - (TILE_HEIGHT * 2.0), CAM_HEIGHT / 2.0),
            false,
            world,
            sprite_sheet_handle.clone(),
        );
        initialise_playground(
            world,
            sprite_sheet_handle.clone(),
            prefab_handle,
            self.level_handle.clone(),
        );
        initialise_camera(world);
    }

    fn fixed_update(&mut self, data: StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        let world = data.world;
        let input = world.read_resource::<InputHandler<StringBindings>>();
        if input.action_is_down("exit").unwrap_or(false) {
            return Trans::Quit;
        }
        Trans::None
    }
}

fn load_sprite_sheet(world: &mut World) -> Handle<SpriteSheet> {
    // Load the sprite sheet necessary to render the graphics.
    // The texture is the pixel data
    // `texture_handle` is a cloneable reference to the texture
    let texture_handle = {
        let loader = world.read_resource::<Loader>();
        let texture_storage = world.read_resource::<AssetStorage<Texture>>();
        loader.load(
            "texture/spritesheet.png",
            ImageFormat(get_image_texure_config()),
            (),
            &texture_storage,
        )
    };

    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
        "texture/spritesheet.ron", // Here we load the associated ron file
        SpriteSheetFormat(texture_handle),
        (),
        &sprite_sheet_store,
    )
}

/// Initialises the ground.
fn initialise_ground(
    world: &mut World,
    sprite_sheet: Handle<SpriteSheet>,
    pos: Vec2,
    tile_size: Handle<Prefab<PlatformCuboid>>,
) {
    let transform = Transform::default();

    // Correctly position the tile.
    let pos = Position(pos);

    // Assign the sprite
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet.clone(),
        sprite_number: 2, // grass is the first sprite in the sprite_sheet
    };

    world
        .create_entity()
        .with(tile_size)
        //.with(PlatformCuboid::new())
        .with(pos)
        .with(transform)
        .with(sprite_render.clone())
        .build();
}

fn initialise_playground(
    world: &mut World,
    sprite_sheet: Handle<SpriteSheet>,
    tile_size: Handle<Prefab<PlatformCuboid>>,
    level_handle: Handle<Level>,
) {
    let tiles;
    {
        let asset = &world.read_resource::<AssetStorage<Level>>();
        let level = asset
            .get(&level_handle)
            .expect("Expected level to be loaded.");
        tiles = level.tiles.clone();
    }

    for tile in tiles {
        initialise_ground(
            world,
            sprite_sheet.clone(),
            tile,
            tile_size.clone(),
        );
    }
}

/// Initialises one tile.
fn initialise_actor(pos: Vec2, player: bool, world: &mut World, sprite_sheet: Handle<SpriteSheet>) {
    let mut transform = Transform::default();

    // Correctly position the paddles.
    transform.set_translation_xyz(pos.x, pos.y, 0.0);

    // Assign the sprite
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet.clone(),
        sprite_number: 1, // grass is the first sprite in the sprite_sheet
    };

    if player {
        world
            .create_entity()
            .with(transform)
            .with(sprite_render.clone())
            .with(Player)
            .with(Grounded(false))
            .with(Position(Vec2::new(pos.x, pos.y)))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            //.with(PlatformCollisionPoints::vertical_line(TILE_HEIGHT / 2.0))
            .with(PlatformCollisionPoints::triangle(TILE_HEIGHT / 2.0))
            .with(Collidee::new())
            .build();
    } else {
        world
            .create_entity()
            .with(transform)
            .with(sprite_render.clone())
            .with(Grounded(false))
            .with(Position(Vec2::new(pos.x, pos.y)))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            //.with(PlatformCollisionPoints::vertical_line(TILE_HEIGHT / 2.0))
            .with(PlatformCollisionPoints::triangle(TILE_HEIGHT / 2.0))
            .with(Collidee::new())
            .build();
    }
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

fn initialise_camera(world: &mut World) {
    // Setup camera in a way that our screen covers whole arena and (0, 0) is in the bottom left.
    let mut transform = Transform::default();
    transform.set_translation_xyz(CAM_WIDTH * 0.5, CAM_HEIGHT * 0.5, 1.0);

    world
        .create_entity()
        .with(Camera::standard_2d(CAM_WIDTH, CAM_HEIGHT))
        .with(transform)
        .build();
}

