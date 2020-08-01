use crate::collider::RTreeCollider;
use crate::system::GravitySystem;
use amethyst_core::ecs::{DispatcherBuilder, World};
use amethyst_core::SystemBundle;
use amethyst_error::Error;
use rstar::RTree;

pub struct PhysicsBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for PhysicsBundle {
    fn build(
        self,
        world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(GravitySystem, "gravity_system", &[]);
        Ok(())
    }
}
