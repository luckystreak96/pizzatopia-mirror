use amethyst_core::ecs::{Component, DenseVecStorage, Entity};
use derivative::Derivative;
use rstar::{RTreeObject, AABB};
use std::ops::{Add, Mul};
use ultraviolet::Vec2;

/// When inserted as a `Resource` into `World`,
/// affects the base strength of gravity on `RigidBody` components.
#[derive(Derivative)]
#[derivative(Default)]
pub struct Gravity {
    #[derivative(Default(value = "-10.0"))]
    pub strength: f32,
}

pub struct CollisionResult {
    pub horizontal: Option<(Entity, f32)>,
    pub vertical: Option<(Entity, f32)>,
}

#[derive(Derivative, Component)]
#[derivative(Default)]
pub struct RigidBody {
    pub velocity: Vec2,
    #[derivative(Default(value = "Vec2::new(0.0, 1.0)"))]
    pub gravity: Vec2,
}

impl RigidBody {
    pub fn project_move(&self, time_scale: f32) -> Vec2 {
        self.velocity.mul(time_scale)
    }

    pub fn stop_at_collisions(&mut self, intersections: Vec<&RTreeCollider>) -> CollisionResult {
        CollisionResult {
            horizontal: None,
            vertical: None,
        }
    }
}

#[derive(Component, Clone)]
pub struct Collider {
    pub position: Vec2,
    pub lower: Vec2,
    pub upper: Vec2,
    pub opaque: bool,
}

impl Collider {
    fn intersects(&self, other: &Self) -> bool {
        (self.lower.x <= other.upper.x && self.upper.x >= other.lower.x)
            && (self.lower.y <= other.upper.y && self.upper.y >= other.lower.y)
    }

    fn project(&self, rigid_body: &RigidBody, time_scale: f32) -> (Vec2, Vec2) {
        let projected = self.position.add(rigid_body.project_move(time_scale));
        (self.lower.add(projected), self.upper.add(projected))
    }
}

pub struct RTreeCollider {
    entity: Entity,
    pub opaque: bool,
    lower: Vec2,
    upper: Vec2,
}

impl PartialEq for RTreeCollider {
    fn eq(&self, other: &Self) -> bool {
        (self.lower.x == other.lower.x)
            && (self.lower.y == other.lower.y)
            && (self.upper.x == other.upper.x)
            && (self.upper.y == other.upper.y)
    }
}

impl RTreeCollider {
    pub fn from_projected(
        entity: Entity,
        collider: &Collider,
        rigid_body: &RigidBody,
        time_scale: f32,
    ) -> Self {
        let (lower, upper) = collider.project(rigid_body, time_scale);
        RTreeCollider {
            entity,
            opaque: collider.opaque,
            lower,
            upper,
        }
    }

    pub fn from_current(entity: Entity, collider: &Collider) -> Self {
        RTreeCollider {
            entity,
            opaque: collider.opaque,
            lower: collider.lower,
            upper: collider.upper,
        }
    }
}

impl RTreeObject for RTreeCollider {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(*self.lower.as_array(), *self.upper.as_array())
    }
}

#[derive(Component)]
pub struct ChildTo {
    pub parent: Entity,
    pub offset: Vec2,
}
