use amethyst::{
    assets::ProgressCounter,
    assets::{AssetStorage, Handle, Loader, PrefabData},
    core::transform::Transform,
    derive::PrefabData,
    ecs::prelude::{Component, DenseVecStorage},
    ecs::Entity,
    ecs::WriteStorage,
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    Error,
};
use serde::{Deserialize, Serialize};

use crate::states::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::CollisionDirection;
use crate::utils::{Vec2, Vec3};
use derivative::Derivative;
use rstar::{RTreeObject, AABB};

#[derive(Clone, Debug, PartialEq)]
pub enum CollisionSideOfBlock {
    Top,
    Bottom,
    Left,
    Right,
}

impl CollisionSideOfBlock {
    pub fn is_horizontal(&self) -> bool {
        match self {
            CollisionSideOfBlock::Left => true,
            CollisionSideOfBlock::Right => true,
            _ => false,
        }
    }

    pub fn is_vertical(&self) -> bool {
        match self {
            CollisionSideOfBlock::Top => true,
            CollisionSideOfBlock::Bottom => true,
            _ => false,
        }
    }
}

pub struct CollideeDetails {
    pub name: String,
    pub position: Vec2,
    pub half_size: Vec2,
    pub new_collider_pos: Vec2,
    pub old_collider_vel: Vec2,
    pub new_collider_vel: Vec2,
    pub num_points_of_collision: i32,
    pub correction: f32,
    pub distance: f32,
    pub side: CollisionSideOfBlock,
}
impl CollideeDetails {
    pub(crate) fn new() -> CollideeDetails {
        CollideeDetails {
            name: String::from(""),
            position: Vec2::new(0.0, 0.0),
            half_size: Vec2::new(0.0, 0.0),
            new_collider_pos: Vec2::new(0.0, 0.0),
            old_collider_vel: Vec2::new(0.0, 0.0),
            new_collider_vel: Vec2::new(0.0, 0.0),
            num_points_of_collision: 0,
            correction: 0.0,
            distance: 0.0,
            side: CollisionSideOfBlock::Top,
        }
    }
}

pub struct Collidee {
    pub horizontal: Option<CollideeDetails>,
    pub vertical: Option<CollideeDetails>,
    pub prev_horizontal: Option<CollideeDetails>,
    pub prev_vertical: Option<CollideeDetails>,
}

impl Collidee {
    pub fn new() -> Collidee {
        Collidee {
            horizontal: None,
            vertical: None,
            prev_horizontal: None,
            prev_vertical: None,
        }
    }

    pub fn both(&self) -> bool {
        self.horizontal.is_some() && self.vertical.is_some()
    }

    pub fn prev_collision_points(&self) -> i32 {
        let mut result = 0;
        if let Some(x) = &self.prev_horizontal {
            result += x.num_points_of_collision;
        }

        if let Some(x) = &self.prev_vertical {
            result += x.num_points_of_collision;
        }
        return result;
    }

    pub fn current_collision_points(&self) -> i32 {
        let mut result = 0;
        if let Some(x) = &self.horizontal {
            result += x.num_points_of_collision;
        }

        if let Some(x) = &self.vertical {
            result += x.num_points_of_collision;
        }
        return result;
    }
}

impl Component for Collidee {
    type Storage = DenseVecStorage<Self>;
}

pub struct Grounded(pub bool);

impl Component for Grounded {
    type Storage = DenseVecStorage<Self>;
}

pub struct Velocity(pub Vec2);

impl Component for Velocity {
    type Storage = DenseVecStorage<Self>;
}

pub struct Sticky(pub bool);

impl Component for Sticky {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone)]
pub struct GravityDirection(pub CollisionDirection);

impl Component for GravityDirection {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone)]
pub struct RTreeEntity {
    pub pos: Vec2,
    pub half_size: Vec2,
    pub entity: Entity,
}

impl RTreeEntity {
    pub fn new(pos: Vec2, half_size: Vec2, entity: Entity) -> RTreeEntity {
        RTreeEntity {
            pos,
            half_size,
            entity,
        }
    }

    fn get_corners(&self) -> ([f32; 2], [f32; 2]) {
        let bottom_left = [self.pos.x - self.half_size.x, self.pos.y - self.half_size.y];
        let top_right = [self.pos.x + self.half_size.x, self.pos.y + self.half_size.y];
        (bottom_left, top_right)
    }
}

impl RTreeObject for RTreeEntity {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let corners = self.get_corners();
        AABB::from_corners(corners.0, corners.1)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct Position(pub Vec3);

impl Position {
    pub fn with_depth(&self, depth: f32) -> Position {
        let mut vec3 = self.0;
        vec3.z = depth;
        Position(vec3)
    }
}

impl Component for Position {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug)]
pub struct CollisionPoint {
    pub point: Vec2,
    pub half_reach: f32,
    pub is_horizontal: bool,
}

impl CollisionPoint {
    pub fn new(point: Vec2, half_reach: f32, is_horizontal: bool) -> CollisionPoint {
        CollisionPoint {
            point,
            half_reach,
            is_horizontal,
        }
    }
}

// The points represent offsets from Position
pub struct PlatformCollisionPoints {
    pub collision_points: Vec<CollisionPoint>,
    pub half_size: Vec2,
}

impl Component for PlatformCollisionPoints {
    type Storage = DenseVecStorage<Self>;
}
impl PlatformCollisionPoints {
    pub fn plus(half_width: f32, half_height: f32) -> PlatformCollisionPoints {
        let mut vec = Vec::new();
        let left = CollisionPoint::new(Vec2::new(-half_width, 0.), half_height, false);
        let right = CollisionPoint::new(Vec2::new(half_width, 0.), half_height, false);
        let top = CollisionPoint::new(Vec2::new(0., half_height), half_width, true);
        let bottom = CollisionPoint::new(Vec2::new(0., -half_height), half_width, true);
        vec.push(left);
        vec.push(right);
        vec.push(top);
        vec.push(bottom);
        PlatformCollisionPoints {
            collision_points: vec,
            half_size: Vec2::new(half_width, half_height),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PrefabData)]
#[prefab(Component)]
#[serde(deny_unknown_fields)]
pub struct PlatformCuboid {
    pub half_width: f32,
    pub half_height: f32,
}

impl PlatformCuboid {
    pub fn new() -> PlatformCuboid {
        PlatformCuboid {
            half_width: TILE_WIDTH / 2.0,
            half_height: TILE_HEIGHT / 2.0,
        }
    }

    pub fn to_vec2(&self) -> Vec2 {
        Vec2::new(self.half_width, self.half_height)
    }

    pub fn create(tile_size_x: f32, tile_size_y: f32) -> PlatformCuboid {
        PlatformCuboid {
            half_width: tile_size_x / 2.0,
            half_height: tile_size_y / 2.0,
        }
    }

    pub fn intersects_point(&self, point: &Vec2, pos: &Vec2) -> bool {
        self.intersect_x(point, pos) && self.intersect_y(point, pos)
    }

    pub(crate) fn intersect_x(&self, point: &Vec2, pos: &Vec2) -> bool {
        self.within_range_x(point, pos, 0.0)
    }

    pub(crate) fn intersect_y(&self, point: &Vec2, pos: &Vec2) -> bool {
        self.within_range_y(point, pos, 0.0)
    }

    pub(crate) fn within_range_x(&self, point: &Vec2, pos: &Vec2, delta: f32) -> bool {
        if point.x <= pos.x + self.half_width + delta && point.x >= pos.x - self.half_width - delta
        {
            return true;
        }
        return false;
    }

    pub(crate) fn within_range_y(&self, point: &Vec2, pos: &Vec2, delta: f32) -> bool {
        if point.y <= pos.y + self.half_height + delta
            && point.y >= pos.y - self.half_height - delta
        {
            return true;
        }
        return false;
    }
}

impl Component for PlatformCuboid {
    type Storage = DenseVecStorage<Self>;
}
