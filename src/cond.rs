use std::sync::{Arc, Mutex};

use bevy::{ecs::system::SystemId, prelude::*, ui::experimental::GhostNode};

use crate::ChildTuple;

#[derive(Component)]
#[require(GhostNode)]
pub struct CondCell(Arc<Mutex<dyn AnyCond + 'static + Sync + Send>>);

trait AnyCond {
    fn update(&mut self, world: &mut World, entity: Entity);
}

pub struct Cond<
    M,
    TestFn: IntoSystem<(), bool, M>,
    Pos: ChildTuple,
    PosFn: Fn() -> Pos,
    Neg: ChildTuple,
    NegFn: Fn() -> Neg,
> {
    state: bool,
    test: Option<TestFn>,
    test_id: Option<SystemId<(), bool>>,
    pos: PosFn,
    neg: NegFn,
    marker: std::marker::PhantomData<M>,
}

impl<
        M: Send + Sync + 'static,
        TestFn: IntoSystem<(), bool, M> + Send + Sync + 'static,
        Pos: ChildTuple + 'static,
        PosFn: Fn() -> Pos + Send + Sync + 'static,
        Neg: ChildTuple + 'static,
        NegFn: Fn() -> Neg + Send + Sync + 'static,
    > Cond<M, TestFn, Pos, PosFn, Neg, NegFn>
{
    #[allow(clippy::new_ret_no_self)]
    pub fn new(test: TestFn, pos: PosFn, neg: NegFn) -> CondCell {
        CondCell(Arc::new(Mutex::new(Self {
            state: false,
            test: Some(test),
            test_id: None,
            pos,
            neg,
            marker: std::marker::PhantomData,
        })))
    }
}

impl<
        M,
        TestFn: IntoSystem<(), bool, M> + 'static,
        Pos: ChildTuple,
        PosFn: Fn() -> Pos,
        Neg: ChildTuple,
        NegFn: Fn() -> Neg,
    > AnyCond for Cond<M, TestFn, Pos, PosFn, Neg, NegFn>
{
    fn update(&mut self, world: &mut World, entity: Entity) {
        let mut first = false;
        if let Some(test) = self.test.take() {
            let test_id = world.register_system(test);
            self.test_id = Some(test_id);
            first = true;
        }

        if let Some(test_id) = self.test_id {
            let test = world.run_system(test_id);
            if let Ok(test) = test {
                if self.state != test || first {
                    let mut entt = world.entity_mut(entity);
                    entt.despawn_descendants();
                    if test {
                        (self.pos)().create(&mut entt);
                    } else {
                        (self.neg)().create(&mut entt);
                    }
                    self.state = test;
                }
            }
        }
    }
}

pub struct CondPlugin;

impl Plugin for CondPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_cond);
    }
}

pub fn update_cond(world: &mut World) {
    let mut query = world.query::<(Entity, &CondCell)>();
    let conditions = query
        .iter(world)
        .map(|(entity, cond)| (entity, cond.0.clone()))
        .collect::<Vec<_>>();
    for (entity, cond) in conditions {
        cond.lock().unwrap().update(world, entity);
    }
}
