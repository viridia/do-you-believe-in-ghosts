use std::sync::{Arc, Mutex};

use bevy::{
    ecs::{system::SystemId, world::DeferredWorld},
    prelude::*,
};

use crate::{
    children::LazyChildTuple,
    effect::{AnyEffect, EffectCell, UnregisterSystemCommand},
};

/// Conditional control-flow node.
pub struct Cond<M, TestFn: IntoSystem<(), bool, M>, Pos: LazyChildTuple, Neg: LazyChildTuple> {
    state: bool,
    test: Option<TestFn>,
    test_id: Option<SystemId<(), bool>>,
    pos: Pos,
    neg: Neg,
    marker: std::marker::PhantomData<M>,
}

impl<
        M: Send + Sync + 'static,
        TestFn: IntoSystem<(), bool, M> + Send + Sync + 'static,
        Pos: LazyChildTuple + Send + Sync + 'static,
        Neg: LazyChildTuple + Send + Sync + 'static,
    > Cond<M, TestFn, Pos, Neg>
{
    #[allow(clippy::new_ret_no_self)]
    pub fn new(test: TestFn, pos: Pos, neg: Neg) -> EffectCell {
        // Wrap in a component
        EffectCell(Arc::new(Mutex::new(Self {
            state: false,
            test: Some(test),
            test_id: None,
            pos,
            neg,
            marker: std::marker::PhantomData,
        })))
    }
}

impl<M, TestFn: IntoSystem<(), bool, M> + 'static, Pos: LazyChildTuple, Neg: LazyChildTuple>
    AnyEffect for Cond<M, TestFn, Pos, Neg>
{
    fn update(&mut self, world: &mut World, entity: Entity) {
        // The first time we run, we need to register the one-shot system.
        let mut first = false;
        if let Some(test) = self.test.take() {
            let test_id = world.register_system(test);
            self.test_id = Some(test_id);
            first = true;
        }

        // Run the condition and see if the result changed.
        if let Some(test_id) = self.test_id {
            let test = world.run_system(test_id);
            if let Ok(test) = test {
                if self.state != test || first {
                    let mut entt = world.entity_mut(entity);
                    entt.despawn_descendants();
                    if test {
                        self.pos.create(&mut entt);
                    } else {
                        self.neg.create(&mut entt);
                    }
                    self.state = test;
                }
            }
        }
    }

    fn cleanup(&self, world: &mut DeferredWorld, _entity: Entity) {
        if let Some(test_id) = self.test_id {
            world.commands().queue(UnregisterSystemCommand(test_id));
        }
    }
}
