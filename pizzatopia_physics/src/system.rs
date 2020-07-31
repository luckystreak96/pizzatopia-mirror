use crate::collider::{ChildTo, Collider, Gravity, RTreeCollider, RigidBody};
use amethyst_core::ecs::{Entities, Join, Read, ReadStorage, System, Write, WriteStorage};
use amethyst_core::Time;
use rstar::{RTree, RTreeObject};
use std::ops::{Add, AddAssign, Mul};

/// Applies gravity's force to a `RigidBody`s `velocity`.
/// The gravitational force is affected by the scale of time,
/// and also by the `RigidBody`s `gravity` multiplier.
#[derive(Default)]
pub struct GravitySystem;

impl<'s> System<'s> for GravitySystem {
    type SystemData = (
        WriteStorage<'s, RigidBody>,
        Read<'s, Time>,
        Read<'s, Gravity>,
    );

    fn run(&mut self, (mut bodies, time, gravity): Self::SystemData) {
        for body in (&mut bodies).join() {
            let gravity = body.gravity.mul(gravity.strength * time.time_scale());
            body.velocity.add_assign(gravity);
        }
    }
}

pub struct CollisionSystem;

impl<'s> System<'s> for CollisionSystem {
    type SystemData = (
        WriteStorage<'s, Collider>,
        WriteStorage<'s, RigidBody>,
        Read<'s, Time>,
        Write<'s, RTree<RTreeCollider>>,
        Entities<'s>,
    );

    fn run(&mut self, (mut colliders, mut bodies, time, mut rtree, entities): Self::SystemData) {
        for (collider, body, entity) in (&mut colliders, &mut bodies, &entities).join() {
            // Remove the current collider from the tree
            rtree.remove(&RTreeCollider::from_current(entity, collider));

            // Get all intersecting envelopes with future pos
            // Filter out non-opaque peeps
            let intersections = rtree.locate_in_envelope_intersecting(
                &RTreeCollider::from_projected(entity, collider, body, time.time_scale())
                    .envelope(),
            );
            let opaque: Vec<&RTreeCollider> =
                intersections.into_iter().filter(|rtc| rtc.opaque).collect();

            // # Do this in file [collider.rs]
            // Find distance to nearest hor/ver colliders like before
            // Adjust new velocity
            let new_velocity = body.stop_at_collisions(opaque);

            // TODO
            // Toss out some collision events with the opaque peeps
            // Get all intersecting envelopes with future pos AGAIN
            // Create collision events

            // Re-insert in correct spot
            rtree.insert(RTreeCollider::from_projected(
                entity,
                collider,
                body,
                time.time_scale(),
            ));
        }
    }
}

/// Moves a `Collider` according to the `RigidBody` velocity.
/// All collision verification should already be complete by now.
pub struct RigidBodyMovementSystem;

impl<'s> System<'s> for RigidBodyMovementSystem {
    type SystemData = (
        WriteStorage<'s, RigidBody>,
        WriteStorage<'s, Collider>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut bodies, mut colliders, time): Self::SystemData) {
        for (body, collider) in (&mut bodies, &mut colliders).join() {
            let projection = body.project_move(time.time_scale());
            collider.position.add_assign(projection);
        }
    }
}

/// Moves a `ChildTo` entity to follow its parent.
pub struct ChildHierarchySystem;

impl<'s> System<'s> for ChildHierarchySystem {
    type SystemData = (
        ReadStorage<'s, ChildTo>,
        WriteStorage<'s, Collider>,
        Entities<'s>,
    );

    fn run(&mut self, (children, mut colliders, entities): Self::SystemData) {
        for (child, entity) in (&children, &entities).join() {
            let parent_pos = colliders.get(child.parent).unwrap().position;
            let child_pos = colliders.get_mut(entity).unwrap();
            child_pos.position = parent_pos.add(child.offset);
        }
    }
}
