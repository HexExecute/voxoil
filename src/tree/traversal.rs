use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use arrayvec::ArrayVec;

use crate::tree::{Tree, branch::Address};

/// An index within a branch's bitmask (0..=63), the child slot only indexes *set bits* in the bitmask.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChildSlot(u8);

/// A single step taken during a traversal of the tree.
///
/// Each step represents moving **from** the parent branch **to** a specific child slot.
///
/// Contains:
/// - `to`: The [`Address`] of the parent branch being traversed from to its children.
/// - `slot`: The [`ChildSlot`] which indexes the next child among the parent's children in the bitmask. This is **not** an absolute index.
#[derive(Clone, Copy)]
pub struct TraversalStep {
    pub to: Address,
    pub slot: ChildSlot,
}

/// A fixed-cacpacity stack of [`TraversalStep`]s representing the path taken from the root to the target node.
///
/// Internally, backed by an [`ArrayVec`] with a 16 step capacity as it would take 16 steps to reach the u32 integer limit which is the max coordinate component size.
///
/// Steps are pushed in root-to-leaf order and can be iterated to replay a traversal operation.
///
/// The stack length will always be <= `Tree::max_depth`.
#[repr(transparent)]
#[derive(Default, Clone)]
pub struct TraversalStack(ArrayVec<TraversalStep, 16>);

/// An immutable context for performing immutable actions (e.g. `get()`) on the [`Tree`] using an already traversed location in the form of a [`TraversalStack`].
///
/// Contains:
/// - `tree`: The immutable reference to the [`Tree`].
/// - `stack`: The [`TraversalStack`] containing the traversed stack and by extension the location of the selected node.
pub struct TraversalContext<'a, L> {
    pub tree: &'a Tree<L>,
    pub stack: TraversalStack,
}

/// An mutable context for performing both immutable and mutable actions (e.g. `get()`, `set()`, and `del()`) on the [`Tree`] using an already traversed location in the form of a [`TraversalStack`].
///
/// Contains:
/// - `tree`: The mutable reference to the [`Tree`].
/// - `stack`: The [`TraversalStack`] containing the traversed stack and by extension the location of the selected node.
pub struct TraversalContextMut<'a, L> {
    pub tree: &'a mut Tree<L>,
    pub stack: TraversalStack,
}

impl ChildSlot {
    pub fn new(slot: u8) -> Option<Self> {
        if slot < 64 { Some(Self(slot)) } else { None }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl<'a, L> TraversalContext<'a, L> {
    pub fn get(&self) {
        todo!()
    }
}

impl<'a, L> TraversalContextMut<'a, L> {
    pub fn get(&self) {
        todo!()
    }

    // Maybe make a way to set a group of leaf children?
    pub fn set(&mut self) {
        todo!()
    }

    pub fn del(&mut self) {
        todo!()
    }
}

impl Deref for ChildSlot {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChildSlot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for TraversalStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraversalStep")
            .field("to", &self.to)
            .field("slot", &self.slot)
            .finish()
    }
}

impl Deref for TraversalStack {
    type Target = ArrayVec<TraversalStep, 16>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TraversalStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for TraversalStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TraversalStack ({} steps):", self.0.len())?;
        let mut peekable = self.0.iter().peekable();
        while let Some(step) = peekable.next() {
            let prefix = if peekable.peek().is_some() {
                "├─"
            } else {
                "└─"
            };
            writeln!(f, "{} {:?}", prefix, step)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::branch::Address;

    #[test]
    fn traversal_stack_debug_output() {
        let mut stack = TraversalStack::default();
        stack.push(TraversalStep {
            to: Address(0x4000000000000000), // A solid fork at index 0
            slot: ChildSlot::new(3).unwrap(),
        });
        stack.push(TraversalStep {
            to: Address(0xC000000000000001), // A solid end at index 1
            slot: ChildSlot::new(5).unwrap(),
        });

        let debug_output = format!("{:?}", stack);

        let expected_output = "\
TraversalStack (2 steps):
├─ TraversalStep { to: Address { raw: 0x4000000000000000, is_end: false, is_solid: true, index: 0x0000000000000000 }, slot: ChildSlot(3) }
└─ TraversalStep { to: Address { raw: 0xC000000000000001, is_end: true, is_solid: true, index: 0x0000000000000001 }, slot: ChildSlot(5) }
";
        assert_eq!(debug_output, expected_output);
    }
}
