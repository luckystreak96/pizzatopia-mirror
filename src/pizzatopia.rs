use crate::audio::{initialise_audio, Sounds};
use crate::components::game::{CollisionEvent, Health, Invincibility};
use crate::components::graphics::AnimationCounter;
use crate::components::physics::{
    Collidee, CollisionSideOfBlock, GravityDirection, Grounded, PlatformCollisionPoints,
    PlatformCuboid, Position, Sticky, Velocity,
};
use crate::components::player::Player;
use crate::events::Events;
use crate::level::Level;
use crate::pizzatopia::SpriteSheetType::{Character, Tiles};
use crate::systems::physics::CollisionDirection;
use crate::utils::Vec2;
use amethyst::derive::EventReader;
use amethyst::input::{is_key_down, InputHandler, StringBindings, VirtualKeyCode};
use amethyst::renderer::rendy::hal::image::{Filter, SamplerInfo, WrapMode};
use amethyst::renderer::rendy::texture::image::{ImageTextureConfig, Repr, TextureKind};
use amethyst::ui::UiEvent;
use amethyst::winit::Event;
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
        EventReader, SystemDesc,
    },
    ecs::prelude::{Component, DenseVecStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use log::info;
use log::warn;
use std::io;

pub const CAM_HEIGHT: f32 = TILE_HEIGHT * 12.0;
pub const CAM_WIDTH: f32 = TILE_WIDTH * 16.0;

pub const TILE_WIDTH: f32 = 128.0;
pub const TILE_HEIGHT: f32 = 128.0;

pub const MAX_FALL_SPEED: f32 = 20.0;
pub const MAX_RUN_SPEED: f32 = 20.0;

pub const FRICTION: f32 = 0.90;

#[repr(u8)]
#[derive(Clone)]
enum SpriteSheetType {
    Tiles = 0,
    Character,
}

#[derive(Debug, EventReader, Clone)]
#[reader(MyEventReader)]
pub enum MyEvents {
    Window(Event),
    Ui(UiEvent),
    App(Events),
}

pub(crate) struct Pizzatopia {
    pub level_handle: Handle<Level>,
    pub platform_size_prefab_handle: Handle<Prefab<PlatformCuboid>>,
    pub spritesheets: Vec<Handle<SpriteSheet>>,
}

impl Pizzatopia {
    fn load_sprite_sheets(&mut self, world: &mut World) {
        self.spritesheets
            .push(load_spritesheet(String::from("texture/tiles"), world));
        self.spritesheets
            .push(load_spritesheet(String::from("texture/spritesheet"), world));
    }

    fn initialize_level(&mut self, world: &mut World) {
        let tiles_sprite_sheet_handle = self.spritesheets[Tiles as usize].clone();
        let actor_sprite_sheet_handle = self.spritesheets[Character as usize].clone();
        let prefab_handle = self.platform_size_prefab_handle.clone();

        world.delete_all();

        initialise_actor(
            Vec2::new(CAM_WIDTH / 2.0, CAM_HEIGHT / 2.0),
            true,
            world,
            actor_sprite_sheet_handle.clone(),
        );
        initialise_actor(
            Vec2::new(CAM_WIDTH / 2.0 - (TILE_HEIGHT * 2.0), CAM_HEIGHT / 2.0),
            false,
            world,
            actor_sprite_sheet_handle.clone(),
        );
        initialise_playground(
            world,
            tiles_sprite_sheet_handle.clone(),
            prefab_handle,
            self.level_handle.clone(),
        );
        initialise_camera(world);
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for Pizzatopia {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        let world = data.world;

        world.register::<PlatformCuboid>();
        world.register::<PlatformCollisionPoints>();
        world.register::<Health>();
        world.register::<Invincibility>();

        // initialise_audio(world);

        self.load_sprite_sheets(world);
        self.initialize_level(world);
    }

    fn handle_event(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
        event: MyEvents,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        let world = &mut data.world;
        if let MyEvents::Window(event) = &event {
            let input = world.read_resource::<InputHandler<StringBindings>>();
            if input.action_is_down("exit").unwrap_or(false) {
                return Trans::Quit;
            }
        }

        if let MyEvents::App(event) = &event {
            match event {
                Events::Reset => {
                    println!("Resetting map...");
                    self.initialize_level(world);
                }
                _ => {}
            }
        }

        // Escape isn't pressed, so we stay in this `State`.
        Trans::None
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        Trans::None
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
        sprite_number: 0, // grass is the first sprite in the sprite_sheet
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
        initialise_ground(world, sprite_sheet.clone(), tile, tile_size.clone());
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
            .with(AnimationCounter(0))
            .with(Player)
            .with(Grounded(false))
            .with(Position(Vec2::new(pos.x, pos.y)))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            //.with(PlatformCollisionPoints::vertical_line(TILE_HEIGHT / 2.0))
            .with(PlatformCollisionPoints::square(TILE_HEIGHT / 2.0))
            .with(Sticky(false))
            .with(GravityDirection(CollisionDirection::FromTop))
            .with(Collidee::new())
            .with(Health(5))
            .with(Invincibility(0))
            .build();
    } else {
        world
            .create_entity()
            .with(transform)
            .with(sprite_render.clone())
            .with(AnimationCounter(0))
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
