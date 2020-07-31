use amethyst_core::ecs::{System, WriteStorage, Read, ReadStorage, Entities, Join};
use amethyst_core::Time;
use crate::collider::{RigidBody, Collider, ChildTo, Gravity};
use ultraviolet::wide::{Add, Mul, AddAssign};

/// Applies gravity's force to a `RigidBody`s `velocity`.
/// The gravitational force is affected by the scale of time,
/// and also by the `RigidBody`s `gravity` multiplier.
pub struct GravitySystem;

impl<'s> System<'s> for GravitySystem {
    type SystemData = (WriteStorage<'s, RigidBody>, Read<'s, Time>, Read<'s, Gravity>, );

    fn run(&mut self, (mut bodies, time, gravity): Self::SystemData) {
        for body in (&mut bodies).join() {
            let gravity = body.gravity.mul(gravity.strength * time.time_scale());
            body.velocity.add_assign(gravity);
        }
    }
}

pub struct CollisionDetectionSystem;

impl<'s> System<'s> for CollisionDetectionSystem {
    type SystemData = (WriteStorage<'s, Collider>, ReadStorage<'s, RigidBody>, Read<'s, Time>, );

    fn run(&mut self, (mut colliders, bodies, time): Self::SystemData) {
        for (collider, body) in (&mut colliders, &bodies).join() {
            collider.projected_movement = body.project_move(time.time_scale());
        }

        /// Remove the current collider from the tree
        /// Get all intersecting envelopes with future pos
        /// Filter out non-opaque peeps
        /// Find nearest hor/ver colliders like before
        /// Re-insert in correct spot
    }
}

/// Moves a `Collider` according to the `RigidBody` velocity.
/// All collision verification should already be complete by now.
pub struct RigidBodyMovementSystem;

impl<'s> System<'s> for RigidBodyMovementSystem {
    type SystemData = (WriteStorage<'s, RigidBody>, WriteStorage<'s, Collider>, Read<'s, Time>, );

    fn run(&mut self, (mut bodies, mut colliders, time): Self::SystemData) {
        for (body, collider) in (&mut bodies, &mut colliders).join() {
            let projection = body.project_move(time.time_scale());
            collider.position.add(projection);
        }
    }
}

/// Moves a `ChildTo` entity to follow its parent.
pub struct ChildHierarchySystem;

impl<'s> System<'s> for ChildHierarchySystem {
    type SystemData = (ReadStorage<'s, ChildTo>, WriteStorage<'s, Collider>, Entities<'s>, );

    fn run(
        &mut self,
        (children, mut colliders, entities): Self::SystemData,
    ) {
        for (child, entity) in (&children, &entities).join() {
            let parent_pos = colliders.get(child.parent).unwrap().position;
            let child_pos = colliders.get_mut(entity).unwrap();
            child_pos.position = parent_pos.add(child.offset);
        }
    }
}
