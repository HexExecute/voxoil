#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use voxoil::pool::{Block, SlabPool};
use voxoil::tree::branch::Branch;
use voxoil::tree::builder::Builder;
use voxoil::tree::{Depth, leaf::Leaf};

fn memory_benchmark(_c: &mut Criterion) {
    #[cfg(feature = "dhat-heap")]
    let profiler = dhat::Profiler::new_heap();

    let mut rng = ChaCha8Rng::seed_from_u64(42); // Deterministic seed for reproducibility
    let max_depth = Depth(4);
    let density = 1.0;
    let leaf = Leaf::new(0);

    let _tree = Builder::new(max_depth)
        .with_capacities(
            (0, 0, 0, 0, 0, 0, 4096 + 64 + 1),
            (0, 0, 0, 0, 0, 0, 262144),
        )
        .filled_random(max_depth, leaf, density, &mut rng)
        .build();

    dbg!(_tree.branches.slabs.0.blocks.capacity());
    dbg!(_tree.branches.slabs.1.blocks.capacity());
    dbg!(_tree.branches.slabs.2.blocks.capacity());
    dbg!(_tree.branches.slabs.3.blocks.capacity());
    dbg!(_tree.branches.slabs.4.blocks.capacity());
    dbg!(_tree.branches.slabs.5.blocks.capacity());
    dbg!(_tree.branches.slabs.6.blocks.capacity());

    dbg!(_tree.leaves.slabs.0.blocks.capacity());
    dbg!(_tree.leaves.slabs.1.blocks.capacity());
    dbg!(_tree.leaves.slabs.2.blocks.capacity());
    dbg!(_tree.leaves.slabs.3.blocks.capacity());
    dbg!(_tree.leaves.slabs.4.blocks.capacity());
    dbg!(_tree.leaves.slabs.5.blocks.capacity());
    dbg!(_tree.leaves.slabs.6.blocks.capacity());

    dbg!(std::mem::size_of::<Block<Leaf<u8>, 64>>());
    dbg!(std::mem::size_of::<Block<Branch, 64>>());

    // TODO: This memory usage is exactly right but the memory usage of the _tree is 3.59x.
    // - This usage multiplier (3.59x) is consistent across different depths.
    // - When the capacities of the vectors are multiplied by their contents' sizes and then added, we also get the ideal/theoretical memory footprint (which is great!).
    // - But, btop and dhat both return a memory footprint 3.59x the size.

    // let _pool1: SlabPool<Branch> = SlabPool::with_capacities((0, 0, 0, 0, 0, 0, 4096 + 64 + 1));
    // let _pool2: SlabPool<Leaf<u8>> = SlabPool::with_capacities((0, 0, 0, 0, 0, 0, 262144));

    #[cfg(feature = "dhat-heap")]
    drop(profiler);
}

criterion_group!(benches, memory_benchmark);
criterion_main!(benches);
