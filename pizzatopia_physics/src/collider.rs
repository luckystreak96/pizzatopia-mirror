use amethyst_core::ecs::{Component, DenseVecStorage, Entity};
use derivative::Derivative;
use rstar::{RTreeObject, AABB};
use std::ops::{Add, Mul};
use ultraviolet::Vec2;
use noisy_float::prelude::*;

/// When inserted as a `Resource` into `World`,
/// affects the base strength of gravity on `RigidBody` components.
#[derive(Derivative)]
#[derivative(Default)]
pub struct Gravity {
    #[derivative(Default(value = "-10.0"))]
    pub strength: f32,
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

    fn collision_velocity_xy(
        &self,
        current_collider: &RTreeCollider,
        intersections: &Vec<&RTreeCollider>,
        time_scale: f32,
    ) -> (Option<(f32, Entity)>, Option<(f32, Entity)>) {
        let projected = self.project_move(time_scale);
        let horizontal = intersections
            .iter()
            .filter(|col| match projected.x > 0.0 {
                true => current_collider.upper.x <= col.lower.x,
                false => current_collider.lower.x >= col.upper.x,
            })
            .min_by_key(|col| match projected.x > 0.0 {
                true => n32(col.lower.x),
                false => n32(-col.upper.x),
            });

        let vertical = intersections
            .iter()
            .filter(|col| match projected.y > 0.0 {
                true => current_collider.upper.y <= col.lower.y,
                false => current_collider.lower.y >= col.upper.y,
            })
            .min_by_key(|col| match projected.y > 0.0 {
                true => n32(col.lower.y),
                false => n32(-col.upper.y),
            });

        let mut result = (None, None);
        if let Some(col) = horizontal {
            let x = match projected.x > 0.0 {
                true => col.lower.x - current_collider.upper.x,
                false => col.upper.x - current_collider.lower.x,
            };
            result.0 = Some((x / time_scale, col.entity));
        }
        if let Some(col) = vertical {
            let y = match projected.y > 0.0 {
                true => col.lower.y - current_collider.upper.y,
                false => col.upper.y - current_collider.lower.y,
            };
            result.1 = Some((y / time_scale, col.entity));
        }
        result
    }

    pub fn collide_with_nearest_axis(
        &mut self,
        current_collider: &RTreeCollider,
        intersections: Vec<&RTreeCollider>,
        time_scale: f32,
    ) -> Option<Entity> {
        let new_vel = self.collision_velocity_xy(current_collider, &intersections, time_scale);
        match new_vel {
            (Some(x), Some(y)) => {
                if x < y {
                    self.velocity.x = x.0;
                    Some(x.1)
                } else {
                    self.velocity.y = y.0;
                    Some(y.1)
                }
            }
            (Some(x), None) => {
                self.velocity.x = x.0;
                Some(x.1)
            }
            (None, Some(y)) => {
                self.velocity.y = y.0;
                Some(y.1)
            }
            (None, None) => None,
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

    pub fn from_current_pos(entity: Entity, collider: &Collider) -> Self {
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
