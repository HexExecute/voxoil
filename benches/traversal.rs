use criterion::{Criterion, criterion_group, criterion_main};
use glam::UVec3;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use voxoil::tree::builder::Builder;
use voxoil::tree::{Depth, leaf::Leaf};

// TODO: We need to confirm that memory usage is what it should be.

fn traversal_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::seed_from_u64(42); // Deterministic seed for reproducibility
    let max_depth = Depth(4);
    let density = 1.0;
    let leaf = Leaf::new(0);

    let tree = Builder::new(max_depth)
        .filled_random(max_depth, leaf, density, &mut rng)
        .build();

    let dimension_size = tree.max_depth.dimension_size();
    let queries: Vec<UVec3> = (0..16384)
        .map(|_| {
            UVec3::new(
                rng.random_range(0..dimension_size),
                rng.random_range(0..dimension_size),
                rng.random_range(0..dimension_size),
            )
        })
        .collect();

    let mut i = 0;
    c.bench_function("traversal", |b| {
        b.iter(|| {
            let coordinate = queries[i % queries.len()];
            i += 1;
            tree.at(coordinate).unwrap();
        });
    });

    dbg!()
}

criterion_group!(benches, traversal_benchmark);
criterion_main!(benches);
