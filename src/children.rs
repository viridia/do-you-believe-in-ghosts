use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::{BuildChildren, Bundle, ChildBuild, Command, Component, Entity, EntityWorldMut},
};

pub trait ChildTuple {
    fn create(self, entity: &mut EntityWorldMut);
}

macro_rules! impl_child_tuple {
    ( $($bundle: ident, $idx: tt);+ ) => {
        impl<$(
            $bundle: Bundle + 'static,
        )+> ChildTuple for ( $( $bundle, )* ) {
            fn create(self: Self, entity: &mut EntityWorldMut) {
                entity.with_children(|parent| {
                    $(
                        parent.spawn(self.$idx);
                    )*
                });
            }
        }
    };
}

impl ChildTuple for () {
    fn create(self, _: &mut EntityWorldMut) {}
}

impl_child_tuple!(B0, 0);
impl_child_tuple!(B0, 0; B1, 1);
impl_child_tuple!(B0, 0; B1, 1; B2, 2);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9; E10, 10);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9; E10, 10; E11, 11);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9; E10, 10; E11, 11; E12, 12);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9; E10, 10; E11, 11; E12, 12; E13, 13);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9; E10, 10; E11, 11; E12, 12; E13, 13; E14, 14);
impl_child_tuple!(B0, 0; B1, 1; B2, 2; B3, 3; B4, 4; B5, 5; B6, 6; B7, 7; B8, 8; B9, 9; E10, 10; E11, 11; E12, 12; E13, 13; E14, 14; E15, 15);

pub struct WithChildren<C: ChildTuple>(pub C);

impl<C: ChildTuple + Send + Sync + 'static> Component for WithChildren<C> {
    /// This is a sparse set component as it's only ever added and removed, never iterated over.
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            world.commands().queue(WithChildrenCommand::<C> {
                entity,
                marker: std::marker::PhantomData,
            });
        });
    }
}

impl<C: ChildTuple + Send + Sync + 'static> WithChildren<C> {
    pub fn new(children: C) -> Self {
        Self(children)
    }

    pub(crate) fn create_children(self, entity: &mut EntityWorldMut) {
        self.0.create(entity);
    }
}

pub struct WithChildrenCommand<C> {
    entity: Entity,
    marker: std::marker::PhantomData<C>,
}

impl<C: ChildTuple + Send + Sync + 'static> Command for WithChildrenCommand<C> {
    fn apply(self, world: &mut bevy::prelude::World) {
        let mut entt = world.entity_mut(self.entity);
        let children = entt.take::<WithChildren<C>>().unwrap();
        children.create_children(&mut entt);
    }
}

pub trait LazyChildTuple {
    fn create(&mut self, entity: &mut EntityWorldMut);
}

impl<C: ChildTuple, F: Fn() -> C> LazyChildTuple for F {
    fn create(&mut self, entity: &mut EntityWorldMut) {
        self().create(entity);
    }
}
