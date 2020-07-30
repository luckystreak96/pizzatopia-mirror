use crate::{
    components::{
        editor::TileLayer,
        graphics::{Scale, SpriteSheetType},
        physics::Position,
    },
    states::{
        editor::EDITOR_GRID_SIZE,
        pizzatopia::{DEPTH_ACTORS, DEPTH_TILES, TILE_HEIGHT, TILE_WIDTH},
    },
    systems::editor::align_cursor_position_with_grid,
};
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::{math::Vector3, transform::Transform},
    ecs::{
        prelude::{Component, DenseVecStorage, NullStorage},
        Entity,
    },
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use derivative::Derivative;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ultraviolet::Vec2;

#[derive(Derivative, Debug, Copy, Clone, Serialize, Deserialize)]
#[derivative(Default)]
pub enum Team {
    GoodGuys,
    BadGuys,
    #[derivative(Default)]
    Neutral,
    Individual(u32),
}
impl Component for Team {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Clone)]
pub enum CollisionEvent {
    // (Entity getting hit id, entity hitting id, damage dealt)
    EnemyCollision(u32, u32, u32),
    ProjectileReflection(u32, Team),
    ProjectileBlock(u32),
    // Picker, pickee
    ItemCollect(u32, u32),
    Talk(String, u32),
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Player(pub bool);
impl Component for Player {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Pickup;
impl Component for Pickup {
    type Storage = NullStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PicksThingsUp {
    pub amount_gathered: u32,
}
impl Component for PicksThingsUp {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Drops(pub u32);
impl Component for Drops {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TimedExistence(pub f32);
impl Component for TimedExistence {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Tile;
impl Component for Tile {
    type Storage = NullStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Reflect;
impl Component for Reflect {
    type Storage = NullStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Block;
impl Component for Block {
    type Storage = NullStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Projectile;
impl Component for Projectile {
    type Storage = NullStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Damage(pub u32);
impl Component for Damage {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Talks {
    pub text: String,
}
impl Component for Talks {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Resettable;
impl Component for Resettable {
    type Storage = NullStorage<Self>;
}

#[derive(Default)]
pub struct Health(pub u32);
impl Component for Health {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct Invincibility(pub f32);
impl Component for Invincibility {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum CameraTarget {
    #[derivative(Default)]
    Player,
    Cursor,
    GameObject(u32),
}

impl Component for CameraTarget {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SpriteRenderData {
    pub(crate) sheet: SpriteSheetType,
    pub(crate) number: usize,
}

impl SpriteRenderData {
    pub fn new(sheet: SpriteSheetType, number: usize) -> Self {
        SpriteRenderData { sheet, number }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SerializedObject {
    pub(crate) object_type: SerializedObjectType,
    #[derivative(Default(value = "Some(Vec2::default())"))]
    pub(crate) pos: Option<Vec2>,
    #[derivative(Default(value = "Some(Vec2::new(128.0, 128.0))"))]
    pub(crate) size: Option<Vec2>,
    #[derivative(Default(value = "Some(SpriteRenderData::default())"))]
    pub(crate) sprite: Option<SpriteRenderData>,
    #[derivative(Default(value = "Some(TileLayer::Middle)"))]
    pub(crate) layer: Option<TileLayer>,
}

impl SerializedObject {
    pub fn next_size(&mut self, position: &mut Vec2, x_axis: bool) {
        let mut size = self.size.unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let change = EDITOR_GRID_SIZE;
        match x_axis {
            true => size.x += change,
            false => size.y += change,
        }
        align_cursor_position_with_grid(position, &size);
        self.size = Some(size);
    }

    pub fn prev_size(&mut self, position: &mut Vec2, x_axis: bool) {
        let mut size = self.size.unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let change = -EDITOR_GRID_SIZE;
        match x_axis {
            true => size.x += change,
            false => size.y += change,
        }
        if size.x > 0.0 && size.y > 0.0 {
            align_cursor_position_with_grid(position, &size);
            self.size = Some(size);
        }
    }
}

impl Component for SerializedObject {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct AnimatedTile {
    pub num_frames: usize,
    pub time_per_frame: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct AnimatedTileComp {
    pub anim: AnimatedTile,
    pub counter: f32,
    pub base_sprite: usize,
}
impl Component for AnimatedTileComp {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum SerializedObjectType {
    #[derivative(Default)]
    StaticTile {
        animation: Option<AnimatedTile>,
    },
    Player {
        is_player: Player,
    },
}

impl Component for SerializedObjectType {
    type Storage = DenseVecStorage<Self>;
}

pub struct SerialHelper {
    pub(crate) transform: Transform,
    pub(crate) sprite_render: SpriteRender,
    pub(crate) layer: TileLayer,
    pub(crate) pos: Position,
    pub(crate) scale: Scale,
    pub(crate) size: Vec2,
}

impl SerialHelper {
    pub fn build(so: &SerializedObject, world: &mut World) -> SerialHelper {
        let sprite = so.sprite.unwrap_or(SpriteRenderData::default());
        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(sprite.sheet as u8)]
            .clone();
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: sprite.number,
        };

        let layer = so.layer.unwrap_or(TileLayer::default());

        // Correctly position the tile.
        let pos = Position(so.pos.unwrap());
        let z = layer.to_z_offset()
            + match so.object_type {
                SerializedObjectType::Player { .. } => DEPTH_ACTORS,
                SerializedObjectType::StaticTile { .. } => DEPTH_TILES,
            };

        // Build tile using GameObject
        let size = so.size.unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let scale = Scale(Vec2::new(size.x / TILE_WIDTH, size.y / TILE_HEIGHT));

        let mut transform = Transform::default();
        transform.set_translation_xyz(pos.0.x, pos.0.y, z);
        transform.set_scale(Vector3::new(scale.0.x, scale.0.y, 1.0));

        SerialHelper {
            sprite_render,
            layer,
            pos,
            scale,
            transform,
            size,
        }
    }
}
