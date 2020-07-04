use crate::animations::{AnimationFactory, AnimationId};
use crate::components::ai;
use crate::components::ai::{BasicShootAi, BasicWalkAi};
use crate::components::editor::{
    EditorFlag, InsertionGameObject, InstanceEntityId, SizeForEditorGrid,
};
use crate::components::game::{
    Damage, Health, Invincibility, Projectile, Reflect, SerializedObject, SerializedObjectType,
    SpriteRenderData, Team, Tile, TimedExistence,
};
use crate::components::game::{Player, Resettable};
use crate::components::graphics::SpriteSheetType;
use crate::components::graphics::{AnimationCounter, CameraLimit, Scale};
use crate::components::physics::{
    Collidee, GravityDirection, Grounded, PlatformCollisionPoints, PlatformCuboid, Position,
    RTreeEntity, Sticky, Velocity,
};
use crate::states::loading::AssetsDir;
use crate::states::pizzatopia::{
    CAM_HEIGHT, CAM_WIDTH, DEPTH_ACTORS, DEPTH_EDITOR, DEPTH_PROJECTILES, DEPTH_TILES, TILE_HEIGHT,
    TILE_WIDTH,
};
use crate::systems::editor::EditorButtonEventSystem;
use crate::systems::physics::CollisionDirection;
use crate::ui::file_picker::{FilePickerFilename, DIR_LEVELS};
use crate::utils::{Vec2, Vec3};
use amethyst::core::components::Parent;
use amethyst::core::math::Vector3;
use amethyst::{
    animation::*,
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabLoader, ProcessingState,
        Processor, ProgressCounter, RonFormat, Source,
    },
    core::transform::Transform,
    core::transform::*,
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
use rstar::{RTree, RTreeObject, AABB};
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::ops::Index;
use std::path::PathBuf;
use std::process::id;

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
        let mut pos = Position(serialized_object.pos.unwrap().to_vec3().clone());
        pos.0.z = DEPTH_TILES;

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
            .with(Transparent)
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
            .with(Transparent)
            .with(EditorFlag)
            .with(Tile)
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(pos.clone().with_depth(DEPTH_EDITOR))
            .with(amethyst::core::Hidden)
            .with(scale.clone())
            .with(SizeForEditorGrid(size.clone()))
            .build();

        return entity.id();
    }

    pub fn initialize_projectile(world: &mut World, pos: &Vec2, vel: &Vec2, team: &Team) -> u32 {
        let mut transform = Transform::default();
        transform.set_translation_xyz(pos.x, pos.y, DEPTH_PROJECTILES);

        let sprite_sheet_type = SpriteSheetType::Tiles;
        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(sprite_sheet_type as u8)]
            .clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 5,
        };

        let position = Position(Vec3::new(pos.x, pos.y, DEPTH_PROJECTILES));

        let size = Vec2::new(TILE_WIDTH, TILE_HEIGHT);
        let scale = Scale(Vec2::new(size.x / TILE_WIDTH, size.y / TILE_HEIGHT));
        let collision_points = PlatformCollisionPoints::plus(size.x / 2.25, size.y / 2.25);
        let mut velocity = Velocity::default();
        velocity.vel = vel.clone();

        // Data common to both editor and entity
        let entity = world
            .create_entity()
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(position.clone())
            .with(scale.clone())
            .with(Transparent)
            .with(velocity)
            .with(collision_points)
            .with(Collidee::new())
            .with(Projectile)
            .with(TimedExistence(10.0))
            .with(team.clone())
            .with(Damage(1))
            .build();

        return entity.id();
    }

    pub fn initialize_damage_box(world: &mut World, pos: &Vec2, size: &Vec2, team: &Team) -> u32 {
        let mut transform = Transform::default();
        transform.set_translation_xyz(pos.x, pos.y, DEPTH_PROJECTILES);

        let sprite_sheet_type = SpriteSheetType::Tiles;
        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(sprite_sheet_type as u8)]
            .clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 5,
        };

        let position = Position(Vec3::new(pos.x, pos.y, DEPTH_PROJECTILES));

        let scale = Scale(Vec2::new(size.x / TILE_WIDTH, size.y / TILE_HEIGHT));
        let collision_points = PlatformCollisionPoints::plus(size.x / 2.25, size.y / 2.25);
        let velocity = Velocity::default();

        // Data common to both editor and entity
        let entity = world
            .create_entity()
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(position.clone())
            .with(scale.clone())
            .with(Transparent)
            .with(velocity)
            .with(Reflect)
            .with(collision_points)
            .with(Collidee::new())
            .with(TimedExistence(0.1))
            .with(team.clone())
            .with(Damage(1))
            .build();

        return entity.id();
    }

    pub fn recalculate_collision_tree(world: &mut World) {
        let mut positions = Vec::new();
        for (entity, pos, platform_cuboid) in (
            &world.entities(),
            &world.read_storage::<Position>(),
            &world.read_storage::<PlatformCuboid>(),
        )
            .join()
        {
            let rtree_entity = RTreeEntity::new(pos.0.to_vec2(), platform_cuboid.to_vec2(), entity);
            positions.push(rtree_entity);
        }
        let tree = RTree::bulk_load(positions);
        world.insert(tree);
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
        transform.set_translation_xyz(pos.x, pos.y, DEPTH_ACTORS);

        let sprite_sheet_type = serialized_object
            .sprite
            .and_then(|sprite| Some(sprite.sheet));
        let sprite_sheet_type = sprite_sheet_type.unwrap_or(SpriteSheetType::Didi);
        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(sprite_sheet_type as u8)]
            .clone();
        // Assign the sprite
        let mut sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 0,
        };

        let position = Position(Vec3::new(pos.x, pos.y, DEPTH_ACTORS));

        let size = serialized_object
            .size
            .unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let scale = Scale(Vec2::new(size.x / TILE_WIDTH, size.y / TILE_HEIGHT));
        let collision_points = PlatformCollisionPoints::plus(size.x / 2.25, size.y / 2.25);

        let animation = match sprite_sheet_type {
            // SpriteSheetType::Animation => {
            //     sprite_render.sprite_number = 3;
            //     AnimationFactory::create_walking_animation(world)
            // }
            _ => AnimationFactory::create_sprite_animation(world),
            // _ => AnimationFactory::create_bob(world, 10.),
        };

        let attack_animation = AnimationFactory::create_bob(world, 10.0);

        // Data common to both editor and entity
        let mut builder = world
            .create_entity()
            .with(animation)
            .with(attack_animation)
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(position.clone())
            .with(scale.clone())
            .with(Transparent)
            .with(GravityDirection(CollisionDirection::FromTop))
            .with(Grounded(false))
            .with(Velocity::default())
            .with(collision_points)
            .with(Collidee::new())
            .with(Health(5))
            .with(Invincibility(0.0));
        // .with(Sticky(false))
        if player {
            builder = builder.with(Player(player)).with(Team::GoodGuys);
        } else {
            builder = builder
                .with(BasicWalkAi::default())
                .with(BasicShootAi::default())
                .with(Team::BadGuys)
                .with(Damage(1));
        }
        let entity = builder.build();

        match sprite_sheet_type {
            SpriteSheetType::Animation => {
                // All the other body parts
                let mut sprite_rarm = sprite_render.clone();
                sprite_rarm.sprite_number = 0;
                let mut sprite_larm = sprite_render.clone();
                sprite_larm.sprite_number = 1;
                let mut sprite_head = sprite_render.clone();
                sprite_head.sprite_number = 2;
                let mut sprite_rleg = sprite_render.clone();
                sprite_rleg.sprite_number = 4;
                let mut sprite_lleg = sprite_render.clone();
                sprite_lleg.sprite_number = 5;
                let left_arm = world
                    .create_entity()
                    .with(Transform::default())
                    .with(Position(Vec3::new(-20., 25., 0.)))
                    .with(Transparent)
                    .with(sprite_larm)
                    .with(Parent { entity })
                    .build();
                let right_arm = world
                    .create_entity()
                    .with(Transform::default())
                    .with(position.with_append_xyz(20., 25., 0.))
                    .with(Transparent)
                    .with(sprite_rarm)
                    .with(Parent { entity })
                    .build();
                let left_leg = world
                    .create_entity()
                    .with(Transform::default())
                    .with(position.with_append_xyz(-10., -25., -1.))
                    .with(Transparent)
                    .with(sprite_lleg)
                    .with(Parent { entity })
                    .build();
                let right_leg = world
                    .create_entity()
                    .with(Transform::default())
                    .with(position.with_append_xyz(10., -25., -1.))
                    .with(Transparent)
                    .with(sprite_rleg)
                    .with(Parent { entity })
                    .build();
                let head = world
                    .create_entity()
                    .with(Transform::default())
                    .with(position.with_append_xyz(0., 40., 0.))
                    .with(Transparent)
                    .with(sprite_head)
                    .with(Parent { entity })
                    .build();

                let mut hierarchy = AnimationHierarchy::new();
                hierarchy.nodes.insert(0, entity);
                hierarchy.nodes.insert(1, left_arm);
                hierarchy.nodes.insert(2, right_arm);
                hierarchy.nodes.insert(3, left_leg);
                hierarchy.nodes.insert(4, right_leg);
                hierarchy.nodes.insert(5, head);

                world
                    .write_storage::<AnimationHierarchy<Transform>>()
                    .insert(entity, hierarchy)
                    .expect("Failed to insert AnimationHierarchy");
            }
            _ => {}
        }

        // create editor entity
        if !ignore_editor {
            world
                .create_entity()
                .with(serialized_object.object_type.clone())
                .with(sprite_sheet_type)
                .with(transform.clone())
                .with(Player(player))
                .with(sprite_render.clone())
                .with(position.clone().with_depth(DEPTH_EDITOR))
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
