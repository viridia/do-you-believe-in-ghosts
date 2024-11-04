use std::sync::{Arc, Mutex};

use bevy::{ecs::system::SystemId, prelude::*};

use crate::{
    children::LazyChildTuple,
    effect::{AnyEffect, EffectCell, UnregisterSystemCommand},
};

/// Conditional control-flow node that implements a C-like "switch" statement.
pub struct Switch<P, M, ValueFn: IntoSystem<(), P, M>> {
    switch_index: usize,
    value_fn: Option<ValueFn>,
    value_sys: Option<SystemId<(), P>>,
    cases: Vec<(P, Box<dyn LazyChildTuple + Send + Sync>)>,
    fallback: Option<Box<dyn LazyChildTuple + Send + Sync>>,
    marker: std::marker::PhantomData<M>,
}

impl<
        P: PartialEq + Send + Sync + 'static,
        M: Send + Sync + 'static,
        ValueFn: IntoSystem<(), P, M> + Send + Sync + 'static,
    > Switch<P, M, ValueFn>
{
    /// Constructs a new switch node.
    pub fn new(test: ValueFn) -> Self {
        // Wrap in a component
        Self {
            switch_index: usize::MAX,
            value_fn: Some(test),
            value_sys: None,
            cases: Vec::new(),
            fallback: None,
            marker: std::marker::PhantomData,
        }
    }

    /// Adds a new switch case.
    pub fn case<F: LazyChildTuple + Send + Sync + 'static>(mut self, value: P, case: F) -> Self {
        self.cases.push((value, Box::new(case)));
        self
    }

    /// Sets the fallback case.
    pub fn fallback<F: LazyChildTuple + Send + Sync + 'static>(mut self, fallback: F) -> Self {
        self.fallback = Some(Box::new(fallback));
        self
    }

    pub fn build(self) -> EffectCell {
        EffectCell(Arc::new(Mutex::new(self)))
    }
}

impl<P: PartialEq + 'static, M, ValueFn: IntoSystem<(), P, M> + 'static> AnyEffect
    for Switch<P, M, ValueFn>
{
    fn update(&mut self, world: &mut World, entity: Entity) {
        // The first time we run, we need to register the one-shot system.
        let mut first = false;
        if let Some(test) = self.value_fn.take() {
            let value_sys = world.register_system(test);
            self.value_sys = Some(value_sys);
            first = true;
        }

        // Run the condition and see if the result changed.
        if let Some(test_id) = self.value_sys {
            let value = world.run_system(test_id);

            if let Ok(value) = value {
                let index = self
                    .cases
                    .iter()
                    .enumerate()
                    .find_map(|(i, f)| if f.0 == value { Some(i) } else { None })
                    .unwrap_or(usize::MAX);

                if self.switch_index != index || first {
                    self.switch_index = index;
                    let mut entt = world.entity_mut(entity);
                    entt.despawn_descendants();
                    if index < self.cases.len() {
                        self.cases[index].1.create(&mut entt);
                    } else if let Some(fallback) = self.fallback.as_mut() {
                        fallback.create(&mut entt);
                    };
                }
            }
        }
    }

    fn cleanup(&mut self, world: &mut bevy::ecs::world::DeferredWorld, _entity: Entity) {
        if let Some(test_id) = self.value_sys {
            world.commands().queue(UnregisterSystemCommand(test_id));
        }
    }
}
