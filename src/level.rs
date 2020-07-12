use crate::{
    animations::{AnimationFactory, AnimationId},
    components::{
        ai,
        ai::{BasicShootAi, BasicWalkAi},
        editor::{EditorFlag, InsertionGameObject, InstanceEntityId, SizeForEditorGrid, TileLayer},
        entity_builder::entity_builder,
        game::{
            Damage, Health, Invincibility, Player, Projectile, Reflect, Resettable, SerialHelper,
            SerializedObject, SerializedObjectType, SpriteRenderData, Team, Tile, TimedExistence,
        },
        graphics::{AnimationCounter, BackgroundParallax, CameraLimit, Scale, SpriteSheetType},
        physics::{
            Collidee, GravityDirection, Grounded, PlatformCollisionPoints, PlatformCuboid,
            Position, RTreeEntity, Sticky, Velocity,
        },
    },
    states::{
        loading::AssetsDir,
        pizzatopia::{
            CAM_HEIGHT, CAM_WIDTH, DEPTH_ACTORS, DEPTH_BACKGROUND, DEPTH_EDITOR, DEPTH_PROJECTILES,
            DEPTH_TILES, TILE_HEIGHT, TILE_WIDTH,
        },
    },
    systems::{editor::EditorButtonEventSystem, physics::CollisionDirection},
    ui::file_picker::{FilePickerFilename, DIR_LEVELS},
    utils::{Vec2, Vec3},
};
use amethyst::{
    animation::*,
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabLoader, ProcessingState,
        Processor, ProgressCounter, RonFormat, Source,
    },
    core::{
        components::Parent,
        math::Vector3,
        transform::{Transform, *},
    },
    ecs::{
        prelude::{Component, DenseVecStorage, EntityBuilder, Join, NullStorage},
        VecStorage,
    },
    error::{format_err, Error, ResultExt},
    prelude::*,
    renderer::{
        palette::{Color, LinSrgba, Srgb, Srgba},
        resources::Tint,
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture, Transparent,
    },
    utils::application_root_dir,
};
use derivative::Derivative;
use log::{error, info, warn};
use rstar::RTree;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, io::Write, ops::Index, path::PathBuf, process::id};

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

    pub fn recalculate_collision_tree(world: &mut World) {
        let mut positions = Vec::new();
        for (entity, pos, platform_cuboid, layer) in (
            &world.entities(),
            &world.read_storage::<Position>(),
            &world.read_storage::<PlatformCuboid>(),
            &world.read_storage::<TileLayer>(),
        )
            .join()
        {
            match layer {
                TileLayer::Middle => {
                    let rtree_entity =
                        RTreeEntity::new(pos.0.to_vec2(), platform_cuboid.to_vec2(), entity);
                    positions.push(rtree_entity);
                }
                _ => (),
            }
        }
        let tree = RTree::bulk_load(positions);
        world.insert(tree);
    }

    // Turn the currently-loaded Level asset into entities
    pub(crate) fn load_level(world: &mut World) {
        let serialized_objects = {
            let asset = &world.read_resource::<AssetStorage<Level>>();
            let level = asset
                .get(&world.read_resource::<Handle<Level>>().clone())
                .unwrap_or(&Level::default())
                .clone();
            level.serialized_objects
        };

        if let Some(serialized_objects) = serialized_objects {
            for mut serialized_object in serialized_objects {
                entity_builder::initialize_serialized_object(world, &mut serialized_object, false);
            }
        }

        Self::recalculate_collision_tree(world);

        Level::calculate_camera_limits(world);
    }

    pub(crate) fn save_level(world: &mut World) {
        let filename = world.read_resource::<FilePickerFilename>().filename.clone();
        if filename.is_empty() {
            error!("Can't save file {} - no filename given", filename);
        }
        let assets_dir = world.read_resource::<AssetsDir>().0.clone();
        let path = assets_dir.join(DIR_LEVELS).join(filename);
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
            serialized_objects.push(entity_builder::entity_to_serialized_object(world, entity));
        }
        level.serialized_objects = match serialized_objects.is_empty() {
            true => None,
            false => Some(serialized_objects),
        };

        // Serialize
        let config = ron::ser::PrettyConfig::default();
        let serialized = match ron::ser::to_string_pretty(&level, config) {
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
            let serialized_object =
                entity_builder::entity_to_serialized_object(world, editor_entity.id());
            let new_instance_id =
                entity_builder::initialize_serialized_object(world, &serialized_object, true);
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
}
