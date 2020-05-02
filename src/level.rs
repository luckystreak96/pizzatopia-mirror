use crate::components::editor::{
    EditorFlag, InsertionGameObject, InstanceEntityId, SizeForEditorGrid,
};
use crate::components::game::{GameObject, Health, Invincibility};
use crate::components::game::{Player, Resettable};
use crate::components::graphics::{AnimationCounter, CameraLimit, Scale};
use crate::components::physics::{
    Collidee, GravityDirection, Grounded, PlatformCollisionPoints, PlatformCuboid, Position,
    Sticky, Velocity,
};
use crate::states::loading::{AssetsDir, LevelPath};
use crate::states::pizzatopia::SpriteSheetType::{Character, Snap, Tiles};
use crate::states::pizzatopia::{CAM_HEIGHT, CAM_WIDTH, DEPTH_ACTORS, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::editor::EditorButtonEventSystem;
use crate::systems::physics::CollisionDirection;
use crate::utils::{Vec2, Vec3};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, ProcessingState, Processor,
        ProgressCounter, Source,
    },
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, Join, NullStorage},
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
use log::{error, warn};
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::ops::Index;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct Level {
    game_objects: Option<Vec<GameObject>>,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[serde(default)]
#[derivative(Default)]
pub struct Tile {
    pub pos: Vec2,
    #[derivative(Default(value = "Tile::default_sprite()"))]
    #[serde(skip_serializing_if = "Tile::is_default_sprite")]
    pub sprite: usize,
    #[derivative(Default(value = "Tile::default_size()"))]
    #[serde(skip_serializing_if = "Tile::is_default_size")]
    pub size: Vec2,
}

impl Component for Tile {
    type Storage = DenseVecStorage<Self>;
}

impl Tile {
    fn default_size() -> Vec2 {
        Vec2::new(TILE_WIDTH, TILE_HEIGHT)
    }
    fn is_default_size(size: &Vec2) -> bool {
        *size == Tile::default_size()
    }
    fn default_sprite() -> usize {
        0
    }
    fn is_default_sprite(sprite: &usize) -> bool {
        *sprite == Tile::default_sprite()
    }
}

impl Level {
    pub fn initialize_game_object(
        world: &mut World,
        game_object: &mut GameObject,
        pos: Option<Vec2>,
        ignore_editor: bool,
    ) -> u32 {
        match game_object {
            GameObject::Player(position, player) => Self::initialize_player(
                pos.unwrap_or(position.0.to_vec2()),
                player.0,
                ignore_editor,
                world,
            ),
            GameObject::StaticTile(tile) => {
                tile.pos = pos.unwrap_or(tile.pos);
                Self::initialize_ground(world, &tile)
            }
        }
    }

    pub(crate) fn calculate_camera_limits(world: &mut World) {
        for (camera, limit) in (
            &world.read_storage::<Camera>(),
            &mut world.write_storage::<CameraLimit>(),
        )
            .join()
        {
            for (pos, _, _) in (
                &world.read_storage::<Position>(),
                &world.read_storage::<EditorFlag>(),
                &world.read_storage::<Tile>(),
            )
                .join()
            {
                limit.left = limit.left.min(pos.0.x);
                limit.right = limit.right.max(pos.0.x);
                limit.bottom = limit.bottom.min(pos.0.y);
                limit.top = limit.top.max(pos.0.y);
            }

            // Only set the right limit dynamically if there's enough space
            limit.right = match (limit.right - limit.left).abs() >= CAM_WIDTH {
                true => limit.right - CAM_WIDTH / 2.0,
                false => limit.left + CAM_WIDTH,
            };
            limit.top = match (limit.top - limit.bottom).abs() >= CAM_HEIGHT {
                // Don't clamp top too hard
                true => limit.top, /* - CAM_HEIGHT / 2.0*/
                false => limit.bottom + CAM_HEIGHT,
            };

            // Add the CAM_SIZE offset
            limit.left += CAM_WIDTH / 2.0;
            limit.bottom += CAM_HEIGHT / 2.0;
        }
    }

    /// Initialises the ground.
    pub fn initialize_ground(world: &mut World, tile: &Tile) -> u32 {
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

        // Create gameplay entity
        let entity = world
            .create_entity()
            .with(tile_size.clone())
            //.with(PlatformCuboid::new())
            .with(pos.clone())
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(scale.clone())
            .build();

        // create editor entity
        let editor_entity = world
            .create_entity()
            .with(GameObject::StaticTile(tile.clone()))
            .with(InstanceEntityId(Some(entity.id())))
            .with(EditorFlag)
            .with(tile.clone())
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(pos.clone())
            .with(amethyst::core::Hidden)
            .with(scale.clone())
            .with(SizeForEditorGrid(tile.size.clone()))
            .build();

        return entity.id();
    }

    // Turn the currently-loaded Level asset into entities
    pub(crate) fn load_level(world: &mut World) {
        let game_objects;
        {
            let asset = &world.read_resource::<AssetStorage<Level>>();
            let level = asset
                .get(&world.read_resource::<Handle<Level>>().clone())
                .expect("Expected level to be loaded.");
            game_objects = level.game_objects.clone();
        }

        if let Some(game_objects) = game_objects {
            for mut game_object in game_objects {
                Self::initialize_game_object(world, &mut game_object, None, false);
            }
        }

        Level::calculate_camera_limits(world);
    }

    // Turns all current entities into a RON file with the current date as a name
    pub(crate) fn save_level(world: &mut World) {
        let filename = world.read_resource::<LevelPath>().0.clone();
        let assets_dir = world.read_resource::<AssetsDir>().0.clone();
        let path = assets_dir.join(filename);
        warn!("Saving level {:?}...", path);

        // Create Level struct
        let mut level: Level = Level::default();

        // Add resettables to level
        let mut game_objects = Vec::new();
        for (game_object, _) in (
            &world.read_storage::<GameObject>(),
            &world.read_storage::<EditorFlag>(),
        )
            .join()
        {
            game_objects.push(game_object.clone());
        }
        level.game_objects = match game_objects.is_empty() {
            true => None,
            false => Some(game_objects),
        };

        // Serialize
        let serialized = match ron::ser::to_string(&level) {
            Ok(x) => x,
            Err(e) => {
                error!("Failed to serialize level for saving.");
                return;
            }
        };
        // Write to file
        let mut file = match File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                error!(
                    "Error saving level in file {:?} with error message:\n{}",
                    &path, e
                );
                return;
            }
        };
        file.write_all(serialized.as_bytes()).unwrap();
    }

    // Remove an entity and its instance entity if it is an editor entity
    pub(crate) fn delete_entity(world: &mut World, id: u32) {
        warn!("Deleting tile {:?}!", id);
        // Get the editor entity
        let editor_entity = world.entities().entity(id);

        // Delete the instance entity using editor entity
        if let Some(instance_id) = world.read_storage::<InstanceEntityId>().get(editor_entity) {
            if let Some(instance_id) = instance_id.0 {
                let instance_entity = world.entities().entity(instance_id);
                match world.entities().delete(instance_entity) {
                    Ok(val) => {}
                    Err(e) => error!("Error deleting instance entity."),
                }
            }
        }
        match world.entities().delete(editor_entity) {
            Ok(val) => {}
            Err(e) => error!("Error deleting editor entity."),
        }
    }

    // Reset the entities in the level to match the editor entity states
    pub(crate) fn reinitialize_level(world: &mut World) {
        let mut resettables = Vec::new();

        {
            let entities = world.entities();
            for (editor_entity, instance_id, game_object, _) in (
                &world.entities(),
                &world.read_storage::<InstanceEntityId>(),
                &world.read_storage::<GameObject>(),
                &world.read_storage::<Resettable>(),
            )
                .join()
            {
                if let Some(id) = instance_id.0 {
                    let instance_entity = entities.entity(id);
                    resettables.push((editor_entity, instance_entity, game_object.clone()));
                }
            }
        }

        // Re-create the entities according to their type
        let mut to_remove = Vec::new();
        for (editor_entity, instance_entity, mut game_object) in resettables {
            to_remove.push(instance_entity);
            let new_instance_id = Self::initialize_game_object(world, &mut game_object, None, true);
            world
                .write_storage::<InstanceEntityId>()
                .get_mut(editor_entity)
                .unwrap()
                .0 = Some(new_instance_id);
        }

        world
            .delete_entities(to_remove.as_slice())
            .expect("Failed to delete entities for reset.");
    }

    /// Initialises a player entity
    pub fn initialize_player(
        pos: Vec2,
        player: bool,
        ignore_editor: bool,
        world: &mut World,
    ) -> u32 {
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

        let position = Position(Vec3::new(pos.x, pos.y, DEPTH_ACTORS));

        let entity;
        let builder = world
            .create_entity()
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(AnimationCounter(0))
            .with(Grounded(false))
            .with(position.clone())
            .with(Velocity(Vec2::new(0.0, 0.0)))
            // 2.25 to fit in 1 block holes
            .with(PlatformCollisionPoints::square(TILE_HEIGHT / 2.25))
            .with(Collidee::new())
            .with(Health(5))
            .with(Invincibility(0))
            // .with(Sticky(false))
            // .with(GravityDirection(CollisionDirection::FromTop))
            .with(Transparent);

        entity = match player {
            true => builder
                .with(GravityDirection(CollisionDirection::FromTop))
                .with(Player(player))
                .build(),
            false => builder.build(),
        };

        // create editor entity
        if !ignore_editor {
            world
                .create_entity()
                .with(GameObject::Player(position.clone(), Player(player)))
                .with(Resettable)
                .with(InstanceEntityId(Some(entity.id())))
                .with(EditorFlag)
                .with(SizeForEditorGrid(Vec2::new(TILE_WIDTH, TILE_HEIGHT)))
                .with(transform.clone())
                .with(sprite_render.clone())
                .with(Position(Vec3::new(pos.x, pos.y, DEPTH_ACTORS)))
                // .with(Tint(Srgba::new(1.0, 1.0, 1.0, 0.5).into()))
                .with(amethyst::core::Hidden)
                .with(Transparent)
                .build();
        }
        return entity.id();
    }
}
