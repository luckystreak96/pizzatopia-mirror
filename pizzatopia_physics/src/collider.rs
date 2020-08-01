use amethyst_core::ecs::{Component, DenseVecStorage, Entity};
use amethyst_core::num::Zero;
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

    // TODO : Split in 2 - The x and y calculations are completely independent
    // TODO : Or just generalize using traits
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
            .filter(|col| {
                projected.y.is_zero()
                    || !projected.x.is_zero()
                        && match projected.x > 0.0 {
                            true => {
                                let percent_distance = Vec2::new(
                                    (col.lower.x - current_collider.upper.x) / projected.x,
                                    (col.lower.y - current_collider.upper.y) / projected.y,
                                );
                                percent_distance.x > percent_distance.y
                                    && percent_distance.x <= 1.0
                                    && percent_distance.x > 0.0
                            }
                            false => {
                                let percent_distance = Vec2::new(
                                    (col.upper.x - current_collider.lower.x) / projected.x,
                                    (col.upper.y - current_collider.lower.y) / projected.y,
                                );
                                percent_distance.x > percent_distance.y
                                    && percent_distance.x <= 1.0
                                    && percent_distance.x > 0.0
                            }
                        }
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
            .filter(|col| {
                projected.x.is_zero()
                    || !projected.y.is_zero()
                        && match projected.y > 0.0 {
                            true => {
                                let percent_distance = Vec2::new(
                                    (col.lower.x - current_collider.upper.x) / projected.x,
                                    (col.lower.y - current_collider.upper.y) / projected.y,
                                );
                                percent_distance.x < percent_distance.y
                                    && percent_distance.y <= 1.0
                                    && percent_distance.y > 0.0
                            }
                            false => {
                                let percent_distance = Vec2::new(
                                    (col.upper.x - current_collider.lower.x) / projected.x,
                                    (col.upper.y - current_collider.lower.y) / projected.y,
                                );
                                percent_distance.x < percent_distance.y
                                    && percent_distance.y <= 1.0
                                    && percent_distance.y > 0.0
                            }
                        }
            })
            .min_by_key(|col| match projected.y > 0.0 {
                true => n32(col.lower.y),
                false => n32(-col.upper.y),
            });

        let mut result = (None, None);
        if let Some(col) = horizontal {
            let x = match projected.x > 0.0 {
                true => col.lower.x - current_collider.upper.x - 0.0001,
                false => col.upper.x - current_collider.lower.x + 0.0001,
            };
            result.0 = Some((x / time_scale, col.entity));
        }
        if let Some(col) = vertical {
            let y = match projected.y > 0.0 {
                true => col.lower.y - current_collider.upper.y - 0.0001,
                false => col.upper.y - current_collider.lower.y + 0.0001,
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
        // TODO : Stop calling this from here - get as 2 params instead
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

    pub fn translated(&self, amount: Vec2) -> Self {
        let mut translated = self.clone();
        translated.lower.add_assign(amount);
        translated.upper.add_assign(amount);
        translated
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
