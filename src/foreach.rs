use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use bevy::{ecs::system::SystemId, prelude::*, ui::experimental::GhostNode};

use crate::{
    children::LazyChildTuple,
    effect::{AnyEffect, UnregisterSystemCommand},
    lcs::lcs,
    ChildTuple, EffectCell,
};

pub struct For {}

impl For {
    pub fn each<
        M: Send + Sync + 'static,
        Item: Send + Sync + 'static + Clone + PartialEq,
        ItemIter: 'static + Iterator<Item = Item>,
        ItemFn: IntoSystem<(), ItemIter, M> + Send + Sync + 'static,
        C: ChildTuple + 'static,
        EachFn: Send + Sync + 'static + Fn(&Item) -> C,
        FallbackFn: LazyChildTuple + Send + Sync + 'static,
    >(
        items_fn: ItemFn,
        each: EachFn,
        fallback: FallbackFn,
    ) -> EffectCell {
        EffectCell(Arc::new(Mutex::new(ForEachEffect {
            items_fn: Some(items_fn),
            item_sys: None,
            cmp: PartialEq::eq,
            each,
            fallback,
            state: Vec::new(),
            marker: std::marker::PhantomData,
        })))
    }

    pub fn each_cmp<
        M: Send + Sync + 'static,
        Item: Send + Sync + 'static + Clone,
        CmpFn: Send + Sync + 'static + Fn(&Item, &Item) -> bool,
        ItemIter: 'static + Iterator<Item = Item>,
        ItemFn: IntoSystem<(), ItemIter, M> + Send + Sync + 'static,
        C: ChildTuple + 'static,
        EachFn: Send + Sync + 'static + Fn(&Item) -> C,
        FallbackFn: LazyChildTuple + Send + Sync + 'static,
    >(
        items_fn: ItemFn,
        cmp: CmpFn,
        each: EachFn,
        fallback: FallbackFn,
    ) -> EffectCell {
        EffectCell(Arc::new(Mutex::new(ForEachEffect {
            items_fn: Some(items_fn),
            item_sys: None,
            cmp,
            each,
            fallback,
            state: Vec::new(),
            marker: std::marker::PhantomData,
        })))
    }

    // pub fn index<F: FnMut()>(self, f: F) {}

    // pub fn index_cmp<F: FnMut()>(self, f: F) {}
}

#[derive(Clone)]
struct ListItem<Item: Clone> {
    child: Entity,
    item: Item,
}

/// A reaction that handles the conditional rendering logic.
struct ForEachEffect<
    M,
    Item: Clone,
    CmpFn: Fn(&Item, &Item) -> bool,
    ItemIter: Iterator<Item = Item>,
    ItemFn: IntoSystem<(), ItemIter, M>,
    C: ChildTuple,
    EachFn: Send + Sync + 'static + Fn(&Item) -> C,
    FallbackFn: LazyChildTuple + Send + Sync + 'static,
> where
    Self: Send + Sync,
{
    items_fn: Option<ItemFn>,
    item_sys: Option<SystemId<(), ItemIter>>,
    cmp: CmpFn,
    each: EachFn,
    fallback: FallbackFn,
    state: Vec<ListItem<Item>>,
    marker: std::marker::PhantomData<M>,
}

impl<
        M: Send + Sync + 'static,
        Item: Clone + Send + Sync + 'static,
        CmpFn: Fn(&Item, &Item) -> bool + Send + Sync + 'static,
        ItemIter: Iterator<Item = Item>,
        ItemFn: IntoSystem<(), ItemIter, M> + Send + Sync + 'static,
        C: ChildTuple,
        EachFn: Send + Sync + 'static + Fn(&Item) -> C,
        FallbackFn: LazyChildTuple + Send + Sync + 'static,
    > ForEachEffect<M, Item, CmpFn, ItemIter, ItemFn, C, EachFn, FallbackFn>
{
    /// Uses the sequence of key values to match the previous array items with the updated
    /// array items. Matching items are patched, other items are inserted or deleted.
    ///
    /// # Arguments
    ///
    /// * `bc` - [`BuildContext`] used to build individual elements.
    /// * `prev_state` - Array of view state elements from previous update.
    /// * `prev_range` - The range of elements we are comparing in `prev_state`.
    /// * `next_state` - Array of view state elements to be built.
    /// * `next_range` - The range of elements we are comparing in `next_state`.
    #[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
    fn build_recursive(
        &self,
        world: &mut World,
        // owner: Entity,
        prev_state: &[ListItem<Item>],
        prev_range: Range<usize>,
        next_items: &[Item],
        next_range: Range<usize>,
        out: &mut Vec<ListItem<Item>>,
    ) {
        // Look for longest common subsequence.
        // prev_start and next_start are *relative to the slice*.
        let (prev_start, next_start, lcs_length) = lcs(
            &prev_state[prev_range.clone()],
            &next_items[next_range.clone()],
            |a, b| (self.cmp)(&a.item, b),
        );

        // If there was nothing in common
        if lcs_length == 0 {
            // Raze old elements
            for i in prev_range {
                let prev = &prev_state[i];
                world.entity_mut(prev.child).despawn_recursive();
            }
            // Build new elements
            for i in next_range {
                let child_id = world.spawn(GhostNode::default()).id();
                (self.each)(&next_items[i]).create(&mut world.entity_mut(child_id));
                out.push(ListItem {
                    child: child_id,
                    item: next_items[i].clone(),
                });
            }
            return;
        }

        // Adjust prev_start and next_start to be relative to the entire state array.
        let prev_start = prev_start + prev_range.start;
        let next_start = next_start + next_range.start;

        // Stuff that precedes the LCS.
        if prev_start > prev_range.start {
            if next_start > next_range.start {
                // Both prev and next have entries before lcs, so recurse
                self.build_recursive(
                    world,
                    // owner,
                    prev_state,
                    prev_range.start..prev_start,
                    next_items,
                    next_range.start..next_start,
                    out,
                )
            } else {
                // Deletions
                for i in prev_range.start..prev_start {
                    let prev = &prev_state[i];
                    world.entity_mut(prev.child).despawn_recursive();
                }
            }
        } else if next_start > next_range.start {
            // Insertions
            for i in next_range.start..next_start {
                let child_id = world.spawn(GhostNode::default()).id();
                (self.each)(&next_items[i]).create(&mut world.entity_mut(child_id));
                out.push(ListItem {
                    child: child_id,
                    item: next_items[i].clone(),
                });
            }
        }

        // For items that match, copy over the view and value.
        for i in 0..lcs_length {
            let prev = &prev_state[prev_start + i];
            out.push(prev.clone());
        }

        // Stuff that follows the LCS.
        let prev_end = prev_start + lcs_length;
        let next_end = next_start + lcs_length;
        if prev_end < prev_range.end {
            if next_end < next_range.end {
                // Both prev and next have entries after lcs, so recurse
                self.build_recursive(
                    world,
                    // owner,
                    prev_state,
                    prev_end..prev_range.end,
                    next_items,
                    next_end..next_range.end,
                    out,
                );
            } else {
                // Deletions
                for i in prev_end..prev_range.end {
                    let prev = &prev_state[i];
                    world.entity_mut(prev.child).despawn_recursive();
                }
            }
        } else if next_end < next_range.end {
            // Insertions
            for i in next_end..next_range.end {
                let child_id = world.spawn(GhostNode::default()).id();
                (self.each)(&next_items[i]).create(&mut world.entity_mut(child_id));
                out.push(ListItem {
                    child: child_id,
                    item: next_items[i].clone(),
                });
            }
        }
    }
}

impl<
        M: Send + Sync + 'static,
        Item: Clone + Send + Sync + 'static,
        CmpFn: Fn(&Item, &Item) -> bool + Send + Sync + 'static,
        ItemIter: Iterator<Item = Item> + 'static,
        ItemFn: IntoSystem<(), ItemIter, M> + Send + Sync + 'static,
        C: ChildTuple,
        EachFn: Send + Sync + 'static + Fn(&Item) -> C,
        FallbackFn: LazyChildTuple + Send + Sync + 'static,
    > AnyEffect for ForEachEffect<M, Item, CmpFn, ItemIter, ItemFn, C, EachFn, FallbackFn>
{
    fn update(&mut self, world: &mut World, parent: Entity) {
        let mut first = false;
        if let Some(items_fn) = self.items_fn.take() {
            self.item_sys = Some(world.register_system(items_fn));
            first = true;
        }

        let Some(items_sys) = self.item_sys else {
            return;
        };

        // Create a reactive context and call the test condition.
        let items: Vec<Item> = match world.run_system(items_sys) {
            Ok(items) => items.collect(),
            Err(_) => Vec::default(),
        };
        let mut next_state: Vec<ListItem<Item>> = Vec::with_capacity(items.len());
        let next_len = items.len();
        let prev_len = self.state.len();

        self.build_recursive(
            world,
            &self.state,
            0..prev_len,
            &items,
            0..next_len,
            &mut next_state,
        );
        let children: Vec<Entity> = next_state.iter().map(|i| i.child).collect();
        self.state = std::mem::take(&mut next_state);

        if next_len == 0 {
            if prev_len > 0 || first {
                world.entity_mut(parent).despawn_descendants();
                self.fallback.create(&mut world.entity_mut(parent));
            }
        } else {
            if prev_len == 0 {
                world.entity_mut(parent).despawn_descendants();
            }
            world.entity_mut(parent).replace_children(&children);
            self.state = next_state;
        }
    }

    fn cleanup(&self, world: &mut bevy::ecs::world::DeferredWorld, _entity: Entity) {
        if let Some(items_sys) = self.item_sys {
            world.commands().queue(UnregisterSystemCommand(items_sys));
        }
    }
}
