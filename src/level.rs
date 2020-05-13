use crate::components::editor::{
    EditorFlag, InsertionGameObject, InstanceEntityId, SizeForEditorGrid,
};
use crate::components::game::{
    Health, Invincibility, SerializedObject, SerializedObjectType, SpriteRenderData, Tile,
};
use crate::components::game::{Player, Resettable};
use crate::components::graphics::SpriteSheetType;
use crate::components::graphics::{AnimationCounter, CameraLimit, Scale};
use crate::components::physics::{
    Collidee, GravityDirection, Grounded, PlatformCollisionPoints, PlatformCuboid, Position,
    Sticky, Velocity,
};
use crate::states::loading::{AssetsDir, LevelPath};
use crate::states::pizzatopia::{CAM_HEIGHT, CAM_WIDTH, DEPTH_ACTORS, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::editor::EditorButtonEventSystem;
use crate::systems::physics::CollisionDirection;
use crate::utils::{Vec2, Vec3};
use amethyst::core::math::Vector3;
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
use log::{error, info, warn};
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::ops::Index;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct Level {
    serialized_objects: Option<Vec<SerializedObject>>,
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

impl Level {
    pub fn initialize_serialized_object(
        world: &mut World,
        serialized_object: &SerializedObject,
        ignore_editor: bool,
    ) -> u32 {
        match serialized_object.object_type {
            SerializedObjectType::Player { .. } => {
                Self::initialize_player(world, serialized_object, ignore_editor)
            }
            SerializedObjectType::StaticTile => Self::initialize_ground(world, serialized_object),
        }
    }

    pub fn entity_to_serialized_object(world: &mut World, id: u32) -> SerializedObject {
        let entity = world.entities().entity(id);
        let object_type = world
            .read_storage::<SerializedObjectType>()
            .get(entity)
            .unwrap()
            .clone();

        let size = world
            .read_storage::<SizeForEditorGrid>()
            .get(entity)
            .unwrap()
            .0
            .clone();
        let position = world
            .read_storage::<Position>()
            .get(entity)
            .unwrap()
            .clone();
        let sprite_render = world
            .read_storage::<SpriteRender>()
            .get(entity)
            .unwrap()
            .clone();
        let sprite_sheet_type = world
            .read_storage::<SpriteSheetType>()
            .get(entity)
            .unwrap()
            .clone();

        let mut result: SerializedObject = SerializedObject::default();
        result.size = Some(size);
        result.pos = Some(position.0.to_vec2());
        result.sprite = Some(SpriteRenderData::new(
            sprite_sheet_type.clone(),
            sprite_render.sprite_number,
        ));

        match object_type {
            SerializedObjectType::StaticTile => {
                result.object_type = SerializedObjectType::StaticTile;
            }
            SerializedObjectType::Player { is_player: _ } => {
                let is_player = world.read_storage::<Player>().get(entity).unwrap().clone();
                result.object_type = SerializedObjectType::Player { is_player };
            }
        };
        result
    }

    pub(crate) fn calculate_camera_limits(world: &mut World) {
        for (_camera, limit) in (
            &world.read_storage::<Camera>(),
            &mut world.write_storage::<CameraLimit>(),
        )
            .join()
        {
            *limit = CameraLimit::default();
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
    pub fn initialize_ground(world: &mut World, serialized_object: &SerializedObject) -> u32 {
        // Build tile using GameObject
        let size = serialized_object
            .size
            .unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let tile_size = PlatformCuboid::create(size.x, size.y);
        let scale = Scale(Vec2::new(size.x / TILE_WIDTH, size.y / TILE_HEIGHT));

        // Correctly position the tile.
        let pos = Position(serialized_object.pos.unwrap().to_vec3().clone());

        let mut transform = Transform::default();
        transform.set_translation_xyz(pos.0.x, pos.0.y, pos.0.z);
        transform.set_scale(Vector3::new(scale.0.x, scale.0.y, 1.0));

        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(serialized_object.sprite.unwrap().sheet as u8)]
            .clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: serialized_object.sprite.unwrap().number, // grass is the first sprite in the sprite_sheet
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
        world
            .create_entity()
            .with(serialized_object.object_type.clone())
            .with(serialized_object.sprite.unwrap().sheet)
            .with(InstanceEntityId(Some(entity.id())))
            .with(EditorFlag)
            .with(Tile)
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(pos.clone())
            .with(amethyst::core::Hidden)
            .with(scale.clone())
            .with(SizeForEditorGrid(size.clone()))
            .build();

        return entity.id();
    }

    // Turn the currently-loaded Level asset into entities
    pub(crate) fn load_level(world: &mut World) {
        let serialized_objects;
        {
            let asset = &world.read_resource::<AssetStorage<Level>>();
            let level = asset
                .get(&world.read_resource::<Handle<Level>>().clone())
                .unwrap_or(&Level::default())
                .clone();
            serialized_objects = level.serialized_objects.clone();
        }

        if let Some(serialized_objects) = serialized_objects {
            for mut serialized_object in serialized_objects {
                Self::initialize_serialized_object(world, &mut serialized_object, false);
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

        // Add GameObjects to level
        let mut entity_ids = Vec::new();
        for (_, entity, _) in (
            &world.read_storage::<SerializedObjectType>(),
            &world.entities(),
            &world.read_storage::<EditorFlag>(),
        )
            .join()
        {
            entity_ids.push(entity.id());
        }

        let mut serialized_objects = Vec::new();
        for entity in entity_ids {
            serialized_objects.push(Self::entity_to_serialized_object(world, entity));
        }
        level.serialized_objects = match serialized_objects.is_empty() {
            true => None,
            false => Some(serialized_objects),
        };

        // Serialize
        let serialized = match ron::ser::to_string(&level) {
            Ok(x) => x,
            Err(e) => {
                error!("Failed to serialize level for saving: {:?}", e);
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
                    Ok(_) => {}
                    Err(_) => error!("Error deleting instance entity."),
                }
            }
        }
        match world.entities().delete(editor_entity) {
            Ok(_) => {}
            Err(_) => error!("Error deleting editor entity."),
        }
    }

    // Reset the entities in the level to match the editor entity states
    pub(crate) fn reinitialize_level(world: &mut World) {
        let mut resettables = Vec::new();

        {
            let entities = world.entities();
            for (editor_entity, instance_id, serialized_object_type, _) in (
                &world.entities(),
                &world.read_storage::<InstanceEntityId>(),
                &world.read_storage::<SerializedObjectType>(),
                &world.read_storage::<Resettable>(),
            )
                .join()
            {
                if let Some(id) = instance_id.0 {
                    let instance_entity = entities.entity(id);
                    resettables.push((
                        editor_entity,
                        instance_entity,
                        serialized_object_type.clone(),
                    ));
                }
            }
        }

        // Re-create the entities according to their type
        let mut to_remove = Vec::new();
        for (editor_entity, instance_entity, _) in resettables {
            to_remove.push(instance_entity);
            let serialized_object = Self::entity_to_serialized_object(world, editor_entity.id());
            let new_instance_id =
                Self::initialize_serialized_object(world, &serialized_object, true);
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
        world: &mut World,
        serialized_object: &SerializedObject,
        ignore_editor: bool,
    ) -> u32 {
        let pos = serialized_object.pos.unwrap_or(Vec2::default());
        let player = match serialized_object.object_type {
            SerializedObjectType::Player { is_player } => is_player.0,
            _ => {
                error!(
                    "Tried to initialize player with the following GameObjectData: {:?}",
                    serialized_object
                );
                false
            }
        };
        let mut transform = Transform::default();
        transform.set_translation_xyz(pos.x, pos.y, 0.0);

        let sprite_sheet_type = serialized_object
            .sprite
            .and_then(|sprite| Some(sprite.sheet));
        let sprite_sheet_type = sprite_sheet_type.unwrap_or(SpriteSheetType::Didi);
        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(sprite_sheet_type as u8)]
            .clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 1,
        };

        let position = Position(Vec3::new(pos.x, pos.y, DEPTH_ACTORS));

        let size = serialized_object
            .size
            .unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let scale = Scale(Vec2::new(size.x / TILE_WIDTH, size.y / TILE_HEIGHT));
        let collision_points = PlatformCollisionPoints::rectangle(size.x / 2.25, size.y / 2.25);

        // Data common to both editor and entity
        let mut builder = world
            .create_entity()
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(position.clone())
            .with(scale.clone())
            .with(Transparent)
            .with(GravityDirection(CollisionDirection::FromTop))
            .with(AnimationCounter(0))
            .with(Grounded(false))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            .with(collision_points)
            .with(Collidee::new())
            .with(Health(5))
            .with(Invincibility(0));
        // .with(Sticky(false))
        if player {
            builder = builder.with(Player(player));
        }
        let entity = builder.build();

        // create editor entity
        if !ignore_editor {
            world
                .create_entity()
                .with(serialized_object.object_type.clone())
                .with(sprite_sheet_type)
                .with(transform.clone())
                .with(Player(player))
                .with(sprite_render.clone())
                .with(position.clone())
                .with(scale.clone())
                .with(Transparent)
                .with(Resettable)
                .with(InstanceEntityId(Some(entity.id())))
                .with(EditorFlag)
                .with(SizeForEditorGrid(Vec2::new(size.x, size.y)))
                // .with(Tint(Srgba::new(1.0, 1.0, 1.0, 0.5).into()))
                .with(amethyst::core::Hidden)
                .build();
        }
        return entity.id();
    }
}
