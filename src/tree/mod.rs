use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use glam::UVec3;

use crate::{
    pool::SlabPool,
    tree::{
        branch::{Address, Branch, ChildMask},
        leaf::Leaf,
        traversal::{TraversalContext, TraversalContextMut, TraversalStack, TraversalStep},
    },
    util::{CoordinatePackError, coordinate_to_slot, localize_coordinate, with_slab_by_count},
};

pub mod branch;
pub mod builder;
pub mod leaf;
pub mod traversal;

/// An error that can occur during traversal of a `Tree`.
#[derive(Debug)]
pub enum TraversalError {
    /// A block was not found in the slab allocator.
    BlockNotFound,
    /// An invalid coordinate was provided.
    InvalidCoordinate(CoordinatePackError),
}

/// A wrapper for a `u8` representing the depth of a tree, branch, or leaf.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Depth(pub u8);

/// A sparse 64-tree data structure for storing and traversing voxel data.
///
/// The tree is composed of branches and leaves. Branches point to other branches or to leaves.
/// Leaves store the actual data.
///
/// The tree is traversed by coordinate, descending from the root branch down to the desired depth.
pub struct Tree<L> {
    // TODO: Temporarily public.
    pub root: Branch,
    pub branches: SlabPool<Branch>,
    pub leaves: SlabPool<Leaf<L>>,
    pub max_depth: Depth,
}

impl<'a, L: Debug> Tree<L> {
    /// Creates a new, empty `Tree` with the given `max_depth`.
    pub fn empty(max_depth: Depth) -> Self {
        Self {
            root: Branch {
                address: Address(0),
                bitmask: ChildMask(0),
            },
            branches: SlabPool::new(),
            leaves: SlabPool::new(),
            max_depth,
        }
    }

    /// Traverses the tree to the given `coordinate` and `max_depth` and returns an immutable [`TraversalContext`].
    ///
    /// The [`TraversalContext`] can be used to get the value at the given coordinate.
    pub fn at_depth(
        &'a self,
        coordinate: UVec3,
        max_depth: Depth,
    ) -> Result<TraversalContext<'a, L>, TraversalError> {
        let mut stack = TraversalStack::default();
        let mut current_branch = self.root;

        for depth in 0..=*max_depth {
            let local_coordinate = localize_coordinate(coordinate, Depth(depth), self.max_depth);
            let slot =
                coordinate_to_slot(local_coordinate).map_err(TraversalError::InvalidCoordinate)?;

            let count = current_branch.bitmask.count();
            if count == 0 {
                // No children, so we can't go deeper. Return the stack as is.
                break;
            }

            let dense_index = current_branch.bitmask.dense_index_of(slot);

            stack.push(TraversalStep {
                to: current_branch.address,
                slot,
            });

            if !current_branch
                .bitmask
                .get(local_coordinate)
                .map_err(TraversalError::InvalidCoordinate)?
            {
                // We're indexing a sparse child slot.
                // Traversal is complete.
                return Ok(TraversalContext { tree: self, stack });
            }

            if current_branch.is_end() {
                // The current_branch points to leaves. We have found the final location.
                // We must validate that the address points to a real block.
                let mut block_found = false;
                with_slab_by_count!(self.leaves, count, |slab| {
                    if slab.get(current_branch.address.index()).is_some() {
                        block_found = true;
                    }
                });
                if !block_found {
                    return Err(TraversalError::BlockNotFound);
                }
                // Traversal is complete.
                return Ok(TraversalContext { tree: self, stack });
            } else {
                // The current_branch points to other branches. We must descend.
                let mut next_branch_opt: Option<Branch> = None;
                let mut block_found = false;

                with_slab_by_count!(self.branches, count, |slab| {
                    // Safely get the block of children from the slab.
                    if let Some(block) = slab.get(current_branch.address.index()) {
                        let next_branch_mu = &block[dense_index];

                        // This is safe under the invariant that a branch with a child bit
                        // set in its mask must point to a valid, initialized block of children.
                        next_branch_opt = Some(unsafe { next_branch_mu.assume_init() });
                        block_found = true;
                    }
                });

                if !block_found {
                    return Err(TraversalError::BlockNotFound);
                }

                current_branch = next_branch_opt.unwrap();
            }
        }

        Ok(TraversalContext { tree: self, stack })
    }

    /// Traverses the tree to the given `coordinate` and `max_depth` and returns a mutable [`TraversalContextMut`].
    ///
    /// The [`TraversalContextMut`] can be used to get, set, or delete the value at the given coordinate.
    #[inline]
    pub fn at_depth_mut(
        &'a mut self,
        coordinate: UVec3,
        max_depth: Depth,
    ) -> Result<TraversalContextMut<'a, L>, TraversalError> {
        Ok(TraversalContextMut {
            stack: self.at_depth(coordinate, max_depth)?.stack,
            tree: self,
        })
    }

    /// Traverses the tree to the given `coordinate` and returns an immutable [`TraversalContext`].
    ///
    /// The [`TraversalContext`] can be used to get the value at the given coordinate.
    #[inline]
    pub fn at(&'a self, coordinate: UVec3) -> Result<TraversalContext<'a, L>, TraversalError> {
        self.at_depth(coordinate, self.max_depth)
    }

    /// Traverses the tree to the given `coordinate` and returns a mutable [`TraversalContextMut`].
    ///
    /// The [`TraversalContextMut`] can be used to get, set, or delete the value at the given coordinate.
    #[inline]
    pub fn at_mut(
        &'a mut self,
        coordinate: UVec3,
    ) -> Result<TraversalContextMut<'a, L>, TraversalError> {
        self.at_depth_mut(coordinate, self.max_depth)
    }

    fn fmt_branch(
        &self,
        branch: &Branch,
        prefix: &str,
        is_last: bool,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let (connector, child_prefix) = if is_last {
            ("└── ", "    ")
        } else {
            ("├── ", "│   ")
        };

        writeln!(f, "{}{}{:?}", prefix, connector, branch)?;

        let new_prefix = format!("{}{}", prefix, child_prefix);
        let count = branch.bitmask.count() as usize;

        if count > 0 {
            if branch.is_end() {
                with_slab_by_count!(self.leaves, count, |slab| {
                    if let Some(block) = slab.get(branch.address.index()) {
                        for (i, leaf) in block.get(0..count).unwrap().iter().enumerate() {
                            let is_last_child = i == count - 1;
                            self.fmt_leaf(
                                unsafe { leaf.assume_init_ref() },
                                &new_prefix,
                                is_last_child,
                                f,
                            )?;
                        }
                    }
                });
            } else {
                with_slab_by_count!(self.branches, count, |slab| {
                    if let Some(block) = slab.get(branch.address.index()) {
                        for (i, child_branch) in block.get(0..count).unwrap().iter().enumerate() {
                            let is_last_child = i == count - 1;
                            self.fmt_branch(
                                unsafe { child_branch.assume_init_ref() },
                                &new_prefix,
                                is_last_child,
                                f,
                            )?;
                        }
                    }
                });
            }
        }

        Ok(())
    }

    fn fmt_leaf(
        &self,
        leaf: &Leaf<L>,
        prefix: &str,
        is_last: bool,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let connector = if is_last { "└─▶ " } else { "├─▶ " };

        writeln!(f, "{}{}{:?}", prefix, connector, leaf)
    }
}

impl Debug for Depth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Depth({})", self.0)
    }
}

impl Deref for Depth {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Depth {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Depth {
    /// Calculates the side length of a voxel at this depth.
    /// This is `1 << (self * 2)` (i.e., `1 << (self << 1)`).
    pub fn dimension_size(&self) -> u32 {
        1 << (self.0 << 1)
    }
}

impl<L: Debug> Debug for Tree<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("max_depth", &self.max_depth)
            .finish()?;

        writeln!(f, "\nTree Structure:")?;
        self.fmt_branch(&self.root, "", true, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_debug_output() {


        
        // 1. Setup the leaf
        let leaf = Leaf::new(123u8);
        let mut leaves_pool = SlabPool::<Leaf<u8>>::new();
        let leaf_block_index = leaves_pool.slabs.0.push([std::mem::MaybeUninit::new(leaf)]);

        // 2. Setup the intermediate branch (end, solid)
        let branch_1 = Branch {
            address: Address((1 << 63) | (1 << 62) | leaf_block_index as u64),
            bitmask: ChildMask(1 << 5), // Child at slot 5
        };

        // 3. Setup the root branch (fork, solid)
        let mut branches_pool = SlabPool::<Branch>::new();
        let branch_block_index = branches_pool
            .slabs
            .0
            .push([std::mem::MaybeUninit::new(branch_1)]);
        let root_branch = Branch {
            address: Address((1 << 62) | branch_block_index as u64),
            bitmask: ChildMask(1 << 3), // Child at slot 3
        };

        // 4. Setup the tree
        let tree: Tree<u8> = Tree {
            root: root_branch,
            branches: branches_pool,
            leaves: leaves_pool,
            max_depth: Depth(1),
        };

        // 5. Print the debug output
        println!("{:#?}", tree);
    }

    #[test]
    fn traversal_one_level() {
        // 1. Setup the leaf
        let leaf = Leaf::new(123u8);
        let mut leaves_pool = SlabPool::<Leaf<u8>>::new();
        let leaf_block_index = leaves_pool.slabs.0.push([std::mem::MaybeUninit::new(leaf)]);

        // 2. Setup the tree
        let root_branch = Branch {
            // Points to the leaf block, is an "end" branch, and is "solid"
            address: Address((1 << 63) | (1 << 62) | leaf_block_index as u64),
            // One child at slot 5
            bitmask: ChildMask(1 << 5),
        };

        let tree: Tree<u8> = Tree {
            root: root_branch,
            branches: SlabPool::new(),
            leaves: leaves_pool,
            max_depth: Depth(0),
        };

        // 3. Calculate coordinate that maps to slot 5 at depth 0
        let coordinate = UVec3::new(1, 1, 0);

        // 4. Traverse and assert
        let context = tree.at(coordinate).unwrap();
        dbg!(&context.stack);
        assert_eq!(context.stack.len(), 1);

        let step = &context.stack[0];
        assert_eq!(step.to, root_branch.address);
        assert_eq!(step.slot.get(), 5);
    }

    #[test]
    fn traversal_two_levels() {
        // 1. Setup the leaf
        let leaf = Leaf::new(123u8);
        let mut leaves_pool = SlabPool::<Leaf<u8>>::new();
        let leaf_block_index = leaves_pool.slabs.0.push([std::mem::MaybeUninit::new(leaf)]);

        // 2. Setup the intermediate branch (end, solid)
        let branch_1 = Branch {
            address: Address((1 << 63) | (1 << 62) | leaf_block_index as u64),
            bitmask: ChildMask(1 << 5), // Child at slot 5
        };

        // 3. Setup the root branch (fork, solid)
        let mut branches_pool = SlabPool::<Branch>::new();
        let branch_block_index = branches_pool
            .slabs
            .0
            .push([std::mem::MaybeUninit::new(branch_1)]);
        let root_branch = Branch {
            address: Address((1 << 62) | branch_block_index as u64),
            bitmask: ChildMask(1 << 3), // Child at slot 3
        };

        // 4. Setup the tree
        let tree: Tree<u8> = Tree {
            root: root_branch,
            branches: branches_pool,
            leaves: leaves_pool,
            max_depth: Depth(1),
        };

        // 5. Calculate coordinate
        // Depth 0 wants slot 3 (0b011) -> local_coord(3,0,0)
        // Depth 1 wants slot 5 (0b101) -> local_coord(1,1,0)
        // With max_depth=1:
        // D0 index = (1*2)-(0*2)=2. local=(c & 12)>>2. For (3,0,0), c should be (12,0,0)
        // D1 index = (1*2)-(1*2)=0. local=(c & 3)>>0. For (1,1,0), c should be (1,1,0)
        // Combined coordinate: x=12|1=13, y=0|1=1, z=0|0=0
        let coordinate = UVec3::new(13, 1, 0);

        // 6. Traverse and assert
        let context = tree.at(coordinate).unwrap();
        println!("\nTraversal stack for two-level traversal:");
        println!("{:?}", context.stack);

        assert_eq!(context.stack.len(), 2);
        assert_eq!(context.stack[0].slot.get(), 3);
        assert_eq!(context.stack[0].to, root_branch.address);
        assert_eq!(context.stack[1].slot.get(), 5);
        assert_eq!(context.stack[1].to, branch_1.address);
    }
}
