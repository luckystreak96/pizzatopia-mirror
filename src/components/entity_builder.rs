pub mod entity_builder {
    use crate::{
        animations::{AnimationFactory, AnimationId},
        components::{
            ai,
            ai::{BasicShootAi, BasicWalkAi},
            editor::{
                CursorWasInThisEntity, EditorFlag, InsertionGameObject, InstanceEntityId,
                RealCursorPosition, SizeForEditorGrid, TileLayer,
            },
            entity_builder::entity_builder,
            game::{
                Damage, Health, Invincibility, Player, Projectile, Reflect, Resettable,
                SerialHelper, SerializedObject, SerializedObjectType, SpriteRenderData, Team, Tile,
                TimedExistence,
            },
            graphics::{
                AbsolutePositioning, AnimationCounter, BackgroundParallax, CameraLimit,
                PulseAnimation, Scale, SpriteSheetType,
            },
            physics::{
                Collidee, GravityDirection, Grounded, PlatformCollisionPoints, PlatformCuboid,
                Position, RTreeEntity, Sticky, Velocity,
            },
        },
        states::{
            loading::AssetsDir,
            pizzatopia,
            pizzatopia::{
                get_camera_center, CAM_HEIGHT, CAM_WIDTH, DEPTH_ACTORS, DEPTH_BACKGROUND,
                DEPTH_EDITOR, DEPTH_PROJECTILES, DEPTH_TILES, DEPTH_UI, TILE_HEIGHT, TILE_WIDTH,
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
            prelude::{Component, DenseVecStorage, NullStorage},
            VecStorage,
        },
        error::{format_err, Error, ResultExt},
        prelude::*,
        renderer::{
            palette::{Color, LinSrgba, Srgb, Srgba},
            resources::Tint,
            Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
            Transparent,
        },
        utils::application_root_dir,
    };

    use log::error;

    use crate::components::game::{AnimatedTile, AnimatedTileComp};
    use std::collections::BTreeMap;

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
        let layer = world
            .read_storage::<TileLayer>()
            .get(entity)
            .unwrap_or(&TileLayer::default())
            .clone();

        let mut result: SerializedObject = SerializedObject::default();
        result.size = Some(size);
        result.pos = Some(position.0.to_vec2());
        result.sprite = Some(SpriteRenderData::new(
            sprite_sheet_type.clone(),
            sprite_render.sprite_number,
        ));
        result.layer = Some(layer);

        match object_type {
            SerializedObjectType::StaticTile { animation } => {
                result.object_type = SerializedObjectType::StaticTile { animation };
            }
            SerializedObjectType::Player { is_player: _ } => {
                let is_player = world.read_storage::<Player>().get(entity).unwrap().clone();
                result.object_type = SerializedObjectType::Player { is_player };
            }
        };
        result
    }

    pub fn initialize_serialized_object(
        world: &mut World,
        serialized_object: &SerializedObject,
        ignore_editor: bool,
    ) -> u32 {
        match serialized_object.object_type {
            SerializedObjectType::Player { .. } => {
                entity_builder::initialize_player(world, serialized_object, ignore_editor)
            }
            SerializedObjectType::StaticTile { .. } => {
                entity_builder::initialize_ground(world, serialized_object)
            }
        }
    }

    pub fn initialize_ground(world: &mut World, serialized_object: &SerializedObject) -> u32 {
        let helper = SerialHelper::build(serialized_object, world);

        let tile_size = PlatformCuboid::create(helper.size.x, helper.size.y);
        let anim = match serialized_object.object_type {
            SerializedObjectType::StaticTile { animation } => animation,
            _ => None,
        }
        .unwrap_or(AnimatedTile::default());
        let animation = AnimatedTileComp {
            anim,
            counter: 0.0,
            base_sprite: helper.sprite_render.sprite_number,
        };

        // Create gameplay entity
        let entity = world
            .create_entity()
            .with(tile_size.clone())
            .with(animation)
            //.with(PlatformCuboid::new())
            .with(Transparent)
            .with(helper.layer)
            .with(helper.pos.clone())
            .with(helper.transform.clone())
            .with(helper.sprite_render.clone())
            .with(helper.scale.clone())
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
            .with(helper.layer)
            .with(helper.transform.clone())
            .with(helper.sprite_render.clone())
            .with(helper.pos.clone().with_depth(helper.pos.0.z + 1.0))
            .with(amethyst::core::Hidden)
            .with(helper.scale.clone())
            .with(SizeForEditorGrid(helper.size.clone()))
            .build();

        return entity.id();
    }

    pub fn initialize_player(
        world: &mut World,
        serialized_object: &SerializedObject,
        ignore_editor: bool,
    ) -> u32 {
        let helper = SerialHelper::build(serialized_object, world);

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

        let collision_points =
            PlatformCollisionPoints::plus(helper.size.x / 2.25, helper.size.y / 2.25);
        let animation = AnimationFactory::create_sprite_animation(world);
        let attack_animation = AnimationFactory::create_bob(world, 10.0);

        let scale = Scale(Vec2::new(1., 1.));

        // Data common to both editor and entity
        let mut builder = world
            .create_entity()
            .with(animation)
            .with(attack_animation)
            .with(helper.transform.clone())
            .with(helper.sprite_render.clone())
            .with(helper.pos.clone())
            .with(scale.clone())
            .with(helper.layer)
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

        // create editor entity
        if !ignore_editor {
            world
                .create_entity()
                .with(serialized_object.object_type.clone())
                .with(
                    serialized_object
                        .sprite
                        .unwrap_or(SpriteRenderData::default())
                        .sheet,
                )
                .with(helper.transform.clone())
                .with(Player(player))
                .with(helper.layer)
                .with(helper.sprite_render.clone())
                .with(helper.pos.clone().with_depth(helper.pos.0.z + 1.0))
                .with(scale.clone())
                .with(Transparent)
                .with(Resettable)
                .with(InstanceEntityId(Some(entity.id())))
                .with(EditorFlag)
                .with(SizeForEditorGrid(Vec2::new(helper.size.x, helper.size.y)))
                // .with(Tint(Srgba::new(1.0, 1.0, 1.0, 0.5).into()))
                .with(amethyst::core::Hidden)
                .build();
        }
        return entity.id();
    }

    pub fn initialize_background(world: &mut World) {
        let size = Vec2::new(2048. / 2., 1024. / 2.);
        let scale = Scale(Vec2::new(CAM_WIDTH / size.x, CAM_HEIGHT / size.y));

        // Correctly position the tile.
        let mut pos = Position(Vec3::new(
            CAM_WIDTH / 2.0,
            CAM_HEIGHT / 2.0,
            DEPTH_BACKGROUND,
        ));

        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(SpriteSheetType::RollingHillsBg as u8)]
            .clone();

        for chain_num in 0..2 {
            for i in 0..4 {
                // Assign the sprite
                let sprite_render = SpriteRender {
                    sprite_sheet: sprite_sheet.clone(),
                    sprite_number: i, // grass is the first sprite in the sprite_sheet
                };
                pos.0.z = DEPTH_BACKGROUND - 0.1 * i as f32;
                let mut transform = Transform::default();
                transform.set_translation_xyz(pos.0.x, pos.0.y, pos.0.z);
                transform.set_scale(Vector3::new(scale.0.x, scale.0.y, 1.0));

                // Create gameplay entity
                world
                    .create_entity()
                    .with(Transparent)
                    .with(transform.clone())
                    .with(pos.clone())
                    .with(scale.clone())
                    .with(sprite_render.clone())
                    .with(BackgroundParallax(i as u32, chain_num))
                    .build();
            }
        }
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
            .with(TimedExistence(0.2))
            .with(team.clone())
            .with(Damage(1))
            .build();

        return entity.id();
    }

    pub fn initialize_cursor(world: &mut World) {
        let mut transform = Transform::default();
        let scale = Vec3::new(0.5, 0.5, 1.0);
        transform.set_scale(Vector3::new(scale.x, scale.y, scale.z));

        // Correctly position the tile.
        let mut pos = get_camera_center(world).to_vec3();
        pos.z = DEPTH_UI;
        let pos = Position(pos);

        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(SpriteSheetType::Tiles as u8)]
            .clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 4,
        };

        // Create cursor
        world
            .create_entity()
            .with(EditorFlag)
            .with(crate::components::editor::EditorCursor::default())
            .with(Tint(Srgba::new(1.0, 1.0, 1.0, 1.0).into()))
            .with(RealCursorPosition(pos.0.to_vec2()))
            .with(PulseAnimation::default())
            .with(Scale(Vec2::new(scale.x, scale.y)))
            .with(SizeForEditorGrid(Vec2::new(
                scale.x * TILE_WIDTH,
                scale.y * TILE_HEIGHT,
            )))
            .with(CursorWasInThisEntity(None))
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(pos.clone())
            .with(AbsolutePositioning)
            .with(Transparent)
            .build();
    }
}
