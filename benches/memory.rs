#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use voxoil::tree::builder::Builder;
use voxoil::tree::{Depth, leaf::Leaf};

fn memory_benchmark(_c: &mut Criterion) {
    #[cfg(feature = "dhat-heap")]
    let profiler = dhat::Profiler::new_heap();

    let mut rng = ChaCha8Rng::seed_from_u64(42); // Deterministic seed for reproducibility.
    let max_depth = Depth(4);
    let density = 1.0;
    let leaf = Leaf::new(0u8);

    let _tree = Builder::new(max_depth)
        .with_capacities(
            (0, 0, 0, 0, 0, 0, 4096 + 64 + 1),
            (0, 0, 0, 0, 0, 0, 262144),
        )
        .filled_random(max_depth, leaf, density, &mut rng)
        .build();

    dbg!(_tree.memory());
    dbg!(_tree.branches.memory());
    dbg!(_tree.leaves.memory());

    #[cfg(feature = "dhat-heap")]
    drop(profiler);
}

criterion_group!(benches, memory_benchmark);
criterion_main!(benches);
