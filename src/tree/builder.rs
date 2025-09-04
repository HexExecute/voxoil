use crate::pool::SlabPool;
use std::fmt::Debug;
use std::path::Path;

use glam::UVec3;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::tree::branch::{Address, Branch, ChildMask};
use crate::tree::leaf::Leaf;
use crate::tree::{Depth, Tree};
use crate::util::{_push_to_slab, push_to_pool};

/// The format of a file to be loaded.
pub enum FileFormat {
    MagicaVoxel,
    /// The Voxoil file format, using the `.vx64` extension.
    Voxoil,
}

/// A type of fractal to generate.
pub enum Fractal {
    MengerSponge,
}

/// A builder for creating a [`Tree`].
pub struct Builder<L> {
    tree: Tree<L>,
}

impl<L: Clone + Debug + Default> Builder<L> {
    /// Creates a new `Builder` with the given `max_depth`.
    pub fn new(max_depth: Depth) -> Self {
        Self {
            tree: Tree::empty(max_depth),
        }
    }

    pub fn with_capacities(
        mut self,
        branch_capacities: (usize, usize, usize, usize, usize, usize, usize),
        leaf_capacities: (usize, usize, usize, usize, usize, usize, usize),
    ) -> Self {
        self.tree.branches = SlabPool::with_capacities(branch_capacities);
        self.tree.leaves = SlabPool::with_capacities(leaf_capacities);
        self
    }

    /// Builds the [`Tree`].
    pub fn build(self) -> Tree<L> {
        self.tree
    }

    /// Fills the [`Tree`] with empty nodes.
    pub fn empty(self, max_depth: Depth) -> Self {
        Self {
            tree: Tree::empty(max_depth),
        }
    }

    /// Fills the [`Tree`] with a single leaf at the given `depth`.
    pub fn filled(self, _leaf: Leaf<L>, _depth: Depth) -> Self {
        todo!()
    }

    pub fn filled_random(
        mut self,
        depth: Depth,
        leaf: Leaf<L>,
        density: f64,
        rng: &mut ChaCha8Rng,
    ) -> Self {
        self.tree.max_depth = depth;
        self.tree.root = self.random_recursive(1, depth, &leaf, density, rng);
        self
    }

    fn random_recursive(
        &mut self,
        current_depth: u8,
        max_depth: Depth,
        leaf: &Leaf<L>,
        density: f64,
        rng: &mut ChaCha8Rng,
    ) -> Branch {
        if current_depth >= *max_depth {
            // Base case: create a leaf if we are at max depth
            let mut leaves = Vec::with_capacity(64);
            let mut bitmask = 0u64;
            for i in 0..64 {
                // TODO: Make these solid, we're only using one leaf, or, make it so that they can choose to also make random leaves (this would require some API architecture planning).
                if rng.random_bool(density) {
                    bitmask |= 1 << i;
                    leaves.push(leaf.clone());
                }
            }

            if leaves.is_empty() {
                return Branch {
                    address: Address(0),
                    bitmask: ChildMask(0),
                };
            }

            let block_index = push_to_pool!(&mut self.tree.leaves, leaves);
            let address = Address((1 << 63) | block_index as u64); // is_end = true

            return Branch {
                address,
                bitmask: ChildMask(bitmask),
            };
        }

        // Recursive step: create a branch
        let mut child_branches = Vec::with_capacity(64);
        let mut bitmask = 0u64;
        for i in 0..64 {
            // TODO: Make a variant that's random per-level its own method.
            // if rng.random_bool(density) {
            let child_branch =
                self.random_recursive(current_depth + 1, max_depth, leaf, density, rng);
            if !child_branch.is_empty() {
                bitmask |= 1 << i;
                child_branches.push(child_branch);
            }
            // }
        }

        if child_branches.is_empty() {
            return Branch {
                address: Address(0),
                bitmask: ChildMask(0),
            };
        }

        let block_index = push_to_pool!(&mut self.tree.branches, child_branches);
        let address = Address(block_index as u64); // is_end = false

        Branch {
            address,
            bitmask: ChildMask(bitmask),
        }
    }

    /// Generates a fractal [`Tree`].
    pub fn fractal(self, _fractal: Fractal) -> Self {
        todo!()
    }

    /// Loads a [`Tree`] from a file.
    pub fn from_file(self, _path: &Path, _format: FileFormat) -> Self {
        todo!()
    }

    /// Generates a [`Tree`] from a signed distance function.
    pub fn from_sdf(self, _sdf: &dyn Fn(UVec3) -> f32) -> Self {
        todo!()
    }
}
