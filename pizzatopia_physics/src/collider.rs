use amethyst_core::ecs::{Component, DenseVecStorage, Entity};
use amethyst_core::num::Zero;
use amethyst_core::Axis2;
use derivative::Derivative;
use noisy_float::prelude::*;
use rstar::{RTreeObject, AABB};
use std::ops::{Add, AddAssign, Mul};
use ultraviolet::Vec2;

/// When inserted as a `Resource` into `World`,
/// affects the base strength of gravity on `RigidBody` components.
#[derive(Derivative)]
#[derivative(Default)]
pub struct Gravity {
    #[derivative(Default(value = "-10.0"))]
    pub strength: f32,
}

#[derive(Derivative, Component, Clone)]
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

    pub fn collide_with_nearest_axis(
        &mut self,
        current_collider: &RTreeCollider,
        intersections: Vec<&RTreeCollider>,
        time_scale: f32,
    ) -> Option<Entity> {
        let collide_x = current_collider.nearest_collider(self, &intersections, time_scale, Axis2::X);
        let collide_y = current_collider.nearest_collider(self, &intersections, time_scale, Axis2::Y);

        let new_vel = (collide_x, collide_y);
        match new_vel {
            (Some(x), Some(y)) => {
                if x < y {
                    self.velocity.x = x.1;
                    Some(x.0)
                } else {
                    self.velocity.y = y.1;
                    Some(y.0)
                }
            }
            (Some(x), None) => {
                self.velocity.x = x.1;
                Some(x.0)
            }
            (None, Some(y)) => {
                self.velocity.y = y.1;
                Some(y.0)
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

#[derive(Clone)]
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
    pub fn new(entity: Entity, collider: &Collider) -> Self {
        RTreeCollider {
            entity,
            opaque: collider.opaque,
            lower: collider.lower.add(collider.position),
            upper: collider.upper.add(collider.position),
        }
    }

    pub(crate) fn translated(&self, amount: Vec2) -> Self {
        let mut translated = self.clone();
        translated.lower.add_assign(amount);
        translated.upper.add_assign(amount);
        translated
    }

    fn can_collide_with_axis(&self, other: &RTreeCollider, axis: usize, velocity: &Vec2) -> bool {
        match velocity.idx(axis) > 0.0 {
            true => self.upper.idx(axis) <= other.lower.idx(axis),
            false => self.lower.idx(axis) >= other.upper.idx(axis),
        }
    }

    fn collides_on_axis(&self, other: &RTreeCollider, axis: usize, velocity: &Vec2) -> bool {
        let a = axis;
        let b = 1 - a;
        velocity.idx(b).is_zero()
            || !velocity.idx(a).is_zero()
                && match velocity.idx(a) > 0.0 {
                    true => {
                        let percent_distance = Vec2::new(
                            (other.lower.x - self.upper.x) / velocity.x,
                            (other.lower.y - self.upper.y) / velocity.y,
                        );
                        percent_distance.idx(a) > percent_distance.idx(b)
                            && percent_distance.idx(a) <= 1.0
                            && percent_distance.idx(a) > 0.0
                    }
                    false => {
                        let percent_distance = Vec2::new(
                            (other.upper.x - self.lower.x) / velocity.x,
                            (other.upper.y - self.lower.y) / velocity.y,
                        );
                        percent_distance.idx(a) > percent_distance.idx(b)
                            && percent_distance.idx(a) <= 1.0
                            && percent_distance.idx(a) > 0.0
                    }
                }
    }

    fn distance_to_collide(&self, other: &RTreeCollider, axis: usize, velocity: &Vec2) -> f32 {
        match velocity.idx(axis) > 0.0 {
            true => other.lower.idx(axis) - self.upper.idx(axis) - 0.0001,
            false => other.upper.idx(axis) - self.lower.idx(axis) + 0.0001,
        }
    }

    fn nearest_collider(
        &self,
        rigid_body: &RigidBody,
        intersections: &Vec<&RTreeCollider>,
        time_scale: f32,
        axis: Axis2,
    ) -> Option<(Entity, f32)> {
        let a = axis as usize;

        let velocity = rigid_body.project_move(time_scale);
        let horizontal = intersections
            .iter()
            .filter(|col| self.can_collide_with_axis(col, a, &velocity))
            .filter(|col| self.collides_on_axis(col, a, &velocity))
            .min_by_key(|col| match velocity.idx(a) > 0.0 {
                true => n32(col.lower.idx(a)),
                false => n32(-col.upper.idx(a)),
            });

        let mut result = None;
        if let Some(col) = horizontal {
            let distance = self.distance_to_collide(col, a, &velocity);
            result = Some((col.entity, distance / time_scale));
        }
        result
    }
}

impl RTreeObject for RTreeCollider {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(*self.lower.as_array(), *self.upper.as_array())
    }
}

trait AxisIndex {
    fn idx(&self, index: usize) -> f32;
}

impl AxisIndex for Vec2 {
    fn idx(&self, index: usize) -> f32 {
        self.as_array()[index]
    }
}

#[derive(Component)]
pub struct ChildTo {
    pub parent: Entity,
    pub offset: Vec2,
}

#[cfg(test)]
mod tests {
    use crate::collider::{Collider, RTreeCollider, RigidBody};
    use amethyst_core::ecs::world::Generation;
    use amethyst_core::ecs::{Builder, Entity, WorldExt};
    use amethyst_core::shred::World;
    use rstar::RTree;
    use std::num::NonZeroI32;
    use ultraviolet::Vec2;

    pub trait EqualsEps {
        fn eq_eps(self, other: Self) -> bool;
    }

    impl EqualsEps for f32 {
        fn eq_eps(self, other: Self) -> bool {
            (self - other).abs() <= 0.01
        }
    }

    #[test]
    fn collision_velocity() {
        let move_collider = Collider {
            position: Vec2::default(),
            lower: Vec2::new(0., 0.),
            upper: Vec2::new(1.0, 1.0),
            opaque: false,
        };
        let mut move_body = RigidBody {
            velocity: Vec2::new(1.75, 1.75),
            gravity: Vec2::default(),
        };

        let obstacle_1 = Collider {
            position: Vec2::new(1.5, 1.0),
            lower: Vec2::new(0., 0.),
            upper: Vec2::new(1.0, 1.0),
            opaque: true,
        };
        let obstacle_2 = Collider {
            position: Vec2::new(0.75, 2.0),
            lower: Vec2::new(0., 0.),
            upper: Vec2::new(1.0, 1.0),
            opaque: true,
        };

        let mut world = World::new();
        world.register::<Collider>();
        world.register::<RigidBody>();
        let moving_entity = world
            .create_entity()
            .with(move_collider.clone())
            .with(move_body.clone())
            .build();
        let obstacle1 = world.create_entity().with(obstacle_1.clone()).build();
        let obstacle2 = world.create_entity().with(obstacle_2.clone()).build();
        let rcollider = RTreeCollider::new(moving_entity, &move_collider);
        let non_refs = vec![
            RTreeCollider::new(obstacle1, &obstacle_1.clone()),
            RTreeCollider::new(obstacle2, &obstacle_2.clone()),
        ];
        let obstacles: Vec<&RTreeCollider> = non_refs.iter().collect();
        let obstacles2 = obstacles.clone();
        move_body.collide_with_nearest_axis(
            &RTreeCollider::new(moving_entity, &move_collider),
            obstacles,
            1.0,
        );
        assert!(move_body.velocity.x.eq_eps(0.5));
        assert!(move_body.velocity.y.eq_eps(1.75));
        move_body.collide_with_nearest_axis(
            &RTreeCollider::new(moving_entity, &move_collider),
            obstacles2,
            1.0,
        );
        assert!(move_body.velocity.x.eq_eps(0.5));
        assert!(move_body.velocity.y.eq_eps(1.0));
    }
}
