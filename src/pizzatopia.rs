use crate::components::physics::{
    Collidee, PlatformCollisionPoints, PlatformCuboid, Position, Velocity,
};
use crate::components::player::Player;
use crate::utils::Vec2;
use amethyst::input::{InputHandler, StringBindings};
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};

pub const CAM_HEIGHT: f32 = 768.0 / 2.0;
pub const CAM_WIDTH: f32 = 512.0;

pub const TILE_WIDTH: f32 = 32.0;
pub const TILE_HEIGHT: f32 = 32.0;

pub const MAX_FALL_SPEED: f32 = 5.0;

pub(crate) struct Pizzatopia;

impl SimpleState for Pizzatopia {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let world = data.world;

        world.register::<PlatformCuboid>();
        world.register::<PlatformCollisionPoints>();

        let sprite_sheet_handle = load_sprite_sheet(world);

        initialise_player(world, sprite_sheet_handle.clone());
        initialise_ground(world, sprite_sheet_handle.clone());
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
            ImageFormat::default(),
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
fn initialise_ground(world: &mut World, sprite_sheet: Handle<SpriteSheet>) {
    let mut transform = Transform::default();

    // Correctly position the paddles.
    let pos = Position(Vec2::new(CAM_WIDTH / 2.0, 0.0));

    // Assign the sprite
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet.clone(),
        sprite_number: 0, // grass is the first sprite in the sprite_sheet
    };

    // Create a left plank entity.
    world
        .create_entity()
        .with(PlatformCuboid::new())
        .with(pos)
        .with(transform)
        .with(sprite_render.clone())
        .build();
}

/// Initialises one tile.
fn initialise_player(world: &mut World, sprite_sheet: Handle<SpriteSheet>) {
    let mut transform = Transform::default();

    // Correctly position the paddles.
    let pos = Vec2::new(CAM_WIDTH / 2.0, CAM_HEIGHT / 2.0);
    transform.set_translation_xyz(pos.x, pos.y, 0.0);

    // Assign the sprite
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet.clone(),
        sprite_number: 1, // grass is the first sprite in the sprite_sheet
    };

    // Create a left plank entity.
    world
        .create_entity()
        .with(transform)
        .with(sprite_render.clone())
        .with(Player)
        .with(Position(Vec2::new(pos.x, pos.y)))
        .with(Velocity(Vec2::new(0.0, 0.0)))
        .with(PlatformCollisionPoints::vertical_line(TILE_HEIGHT / 2.0))
        .with(Collidee::new())
        .build();
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
