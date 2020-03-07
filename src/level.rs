use crate::components::editor::{EditorEntity, SizeForEditorGrid};
use crate::components::game::{Health, Invincibility, Resettable};
use crate::components::graphics::{AnimationCounter, Scale};
use crate::components::physics::{
    Collidee, GravityDirection, Grounded, PlatformCollisionPoints, PlatformCuboid, Position,
    Sticky, Velocity,
};
use crate::components::player::Player;
use crate::states::pizzatopia::SpriteSheetType::{Character, Snap, Tiles};
use crate::states::pizzatopia::{DEPTH_ACTORS, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::CollisionDirection;
use crate::utils::{Vec2, Vec3};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, ProcessingState, Processor,
        ProgressCounter, Source,
    },
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    ecs::VecStorage,
    error::{format_err, Error, ResultExt},
    prelude::*,
    renderer::palette::Color,
    renderer::palette::{LinSrgba, Srgb, Srgba},
    renderer::resources::Tint,
    renderer::Transparent,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    utils::application_root_dir,
};
use derivative::Derivative;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Level {
    tiles: Vec<Tile>,
}

impl Asset for Level {
    const NAME: &'static str = "pizzatopia::level::Level";
    // use `Self` if the type is directly serialized.
    type Data = Self;
    type HandleStorage = VecStorage<Handle<Level>>;
}

impl From<Level> for Result<Level, Error> {
    fn from(level: Level) -> Result<Level, Error> {
        Ok(level)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[serde(default)]
#[derivative(Default)]
pub(crate) struct Tile {
    pos: Vec2,
    sprite: usize,
    #[derivative(Default(value = "Vec2::new(TILE_WIDTH, TILE_HEIGHT)"))]
    size: Vec2,
}

impl Component for Tile {
    type Storage = DenseVecStorage<Self>;
}

impl Level {
    /// Initialises the ground.
    fn initialize_ground(world: &mut World, tile: &Tile) {
        // let tile_size = (*world.read_resource::<Handle<Prefab<PlatformCuboid>>>()).clone();
        let tile_size = PlatformCuboid::create(tile.size.x, tile.size.y);
        let scale = Scale(Vec2::new(
            tile.size.x / TILE_WIDTH,
            tile.size.y / TILE_HEIGHT,
        ));

        let transform = Transform::default();

        // Correctly position the tile.
        let pos = Position(tile.pos.to_vec3().clone());

        let sprite_sheet =
            world.read_resource::<Vec<Handle<SpriteSheet>>>()[Tiles as usize].clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: tile.sprite, // grass is the first sprite in the sprite_sheet
        };

        // create editor entity
        world
            .create_entity()
            .with(EditorEntity)
            .with(tile.clone())
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(pos.clone())
            .with(amethyst::core::Hidden)
            .with(scale.clone())
            .with(SizeForEditorGrid(tile.size.clone()))
            .build();

        // Create gameplay entity
        world
            .create_entity()
            .with(tile_size.clone())
            //.with(PlatformCuboid::new())
            .with(pos)
            .with(transform)
            .with(sprite_render.clone())
            .with(scale.clone())
            .build();
    }

    pub(crate) fn initialize_level(world: &mut World) {
        let tiles;
        {
            let asset = &world.read_resource::<AssetStorage<Level>>();
            let level = asset
                .get(&world.read_resource::<Handle<Level>>().clone())
                .expect("Expected level to be loaded.");
            tiles = level.tiles.clone();
        }

        for tile in tiles {
            Self::initialize_ground(world, &tile);
        }
    }

    /// Initialises one tile.
    pub fn initialise_actor(pos: Vec2, player: bool, world: &mut World) {
        let mut transform = Transform::default();
        transform.set_translation_xyz(pos.x, pos.y, 0.0);

        let sprite_sheet;
        if player {
            sprite_sheet =
                world.read_resource::<Vec<Handle<SpriteSheet>>>()[Character as usize].clone();
        } else {
            sprite_sheet = world.read_resource::<Vec<Handle<SpriteSheet>>>()[Snap as usize].clone();
        }
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 1,
        };

        // create editor entity
        world
            .create_entity()
            .with(EditorEntity)
            .with(SizeForEditorGrid(Vec2::new(TILE_WIDTH, TILE_HEIGHT)))
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(Position(Vec3::new(pos.x, pos.y, DEPTH_ACTORS)))
            // .with(Tint(Srgba::new(1.0, 1.0, 1.0, 0.5).into()))
            .with(amethyst::core::Hidden)
            .with(Transparent)
            .build();

        let builder = world
            .create_entity()
            .with(Resettable)
            .with(transform)
            .with(sprite_render.clone())
            .with(AnimationCounter(0))
            .with(Grounded(false))
            .with(Position(Vec3::new(pos.x, pos.y, DEPTH_ACTORS)))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            .with(PlatformCollisionPoints::square(TILE_HEIGHT / 2.0))
            .with(Collidee::new())
            .with(Health(5))
            .with(Invincibility(0))
            // .with(Sticky(false))
            // .with(GravityDirection(CollisionDirection::FromTop))
            .with(Transparent);

        if player {
            builder
                .with(GravityDirection(CollisionDirection::FromTop))
                .with(Player)
                .build();
        } else {
            builder.build();
        }
    }
}
