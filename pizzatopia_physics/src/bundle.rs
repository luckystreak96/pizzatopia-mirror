use amethyst_core::SystemBundle;
use amethyst_core::ecs::{World, DispatcherBuilder};
use amethyst_error::Error;
use crate::collider::Gravity;

pub struct PhysicsBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for PhysicsBundle {
    fn build(
        self,
        world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        // Set gravity strength
        world.insert(Gravity { strength: -10. });

        dispatcher.add(
            InputManagementSystem::<B>::default(),
            "input_management_system",
            &[],
        );
        Ok(())
    }
}
