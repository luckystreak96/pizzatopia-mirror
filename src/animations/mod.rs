use amethyst::animation::*;
use amethyst::assets::*;
use amethyst::core::Transform;
use amethyst::ecs::prelude::Entity;
use amethyst::ecs::{ReadStorage, WriteStorage};
use amethyst::prelude::{World, WorldExt};
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialOrd, PartialEq, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum AnimationId {
    Scale,
    Rotate,
    Translate,
}

pub enum AnimationAction {
    StartAnimationOrSetRate(f32),
    SetRate(f32),
    AbortAnimation,
}

pub enum SamplerAction {
    SetRate(f32),
}

pub struct AnimationFactory;
impl AnimationFactory {
    pub fn create_bob(world: &World, amplitude: f32) -> AnimationSet<AnimationId, Transform> {
        let mut anim: AnimationSet<AnimationId, Transform> = AnimationSet::default();
        let loader = world.read_resource::<Loader>();
        let sampler = loader.load_from_data(
            Sampler {
                input: vec![0., 1., 2.],
                output: vec![
                    SamplerPrimitive::Vec3([0., 0., 0.]),
                    SamplerPrimitive::Vec3([0., amplitude, 0.]),
                    SamplerPrimitive::Vec3([0., 0., 0.]),
                ],
                function: InterpolationFunction::Linear,
            },
            (),
            &world.read_resource(),
        );
        let animation = loader.load_from_data(
            Animation::new_single(0, TransformChannel::Translation, sampler),
            (),
            &world.read_resource(),
        );
        anim.animations.insert(AnimationId::Translate, animation);
        anim
    }

    pub fn set_animation(
        animation_sets: &ReadStorage<'_, AnimationSet<AnimationId, Transform>>,
        control_sets: &mut WriteStorage<'_, AnimationControlSet<AnimationId, Transform>>,
        target_entity: Entity,
        id: AnimationId,
        state: AnimationAction,
        defer: Option<(AnimationId, DeferStartRelation)>,
    ) {
        let animation = animation_sets
            .get(target_entity)
            .and_then(|s| s.get(&id))
            .cloned()
            .unwrap();
        let sets = control_sets;
        let control_set = get_animation_set::<AnimationId, Transform>(sets, target_entity).unwrap();
        let mut state = state;
        if control_set.has_animation(id) {
            state = match state {
                AnimationAction::StartAnimationOrSetRate(rate) => AnimationAction::SetRate(rate),
                _ => state,
            };
        }
        match state {
            AnimationAction::StartAnimationOrSetRate(rate) => match defer {
                None => {
                    control_set.add_animation(
                        id,
                        &animation,
                        EndControl::Normal,
                        rate,
                        AnimationCommand::Start,
                    );
                }

                Some((defer_id, defer_relation)) => {
                    control_set.add_deferred_animation(
                        id,
                        &animation,
                        EndControl::Normal,
                        rate,
                        AnimationCommand::Start,
                        defer_id,
                        defer_relation,
                    );
                }
            },
            AnimationAction::AbortAnimation => {
                control_set.abort(id);
            }
            AnimationAction::SetRate(rate) => {
                control_set.set_rate(id, rate);
            }
        }
    }
}
