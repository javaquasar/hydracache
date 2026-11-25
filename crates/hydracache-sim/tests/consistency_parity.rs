//! The simulator mirrors the product consistency rules (the product `grid`
//! module is not built for wasm32). Prove the mirror matches
//! `hydracache::ConsistencyLevel` for every cluster size and live set the
//! simulator can reach, including the unsatisfiable reason text.

use std::collections::{BTreeMap, BTreeSet};

use hydracache::{ClusterNodeId, ConsistencyLevel, EffectiveReplicationMap, RegionId, Replicas};
use hydracache_sim::{consistency_level_views, SIM_CONSISTENCY_LEVELS};

const PRODUCT_LEVELS: [ConsistencyLevel; 5] = [
    ConsistencyLevel::One,
    ConsistencyLevel::LocalQuorum,
    ConsistencyLevel::Quorum,
    ConsistencyLevel::EachQuorum,
    ConsistencyLevel::All,
];

#[test]
fn simulator_consistency_mirror_matches_product_rules() {
    assert_eq!(SIM_CONSISTENCY_LEVELS.len(), PRODUCT_LEVELS.len());
    for total in 1..=7_usize {
        let nodes = (0..total)
            .map(|index| ClusterNodeId::from(format!("node-{index}")))
            .collect::<Vec<_>>();
        let map =
            EffectiveReplicationMap::new(Replicas::new(nodes[0].clone(), nodes[1..].to_vec()));
        let region = RegionId::new("local");
        let topology = nodes
            .iter()
            .map(|node| (node.clone(), region.clone()))
            .collect::<BTreeMap<_, _>>();
        // Every subset of live nodes: satisfiability depends on the count only,
        // and checking subsets proves that.
        for mask in 0..(1_u32 << total) {
            let live = nodes
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, node)| node.clone())
                .collect::<BTreeSet<_>>();
            let mirror = consistency_level_views(total, live.len());
            for (view, level) in mirror.iter().zip(PRODUCT_LEVELS) {
                let requirement = level.required_acks(&map, &topology, &region);
                let product = level.validate(&map, &topology, &region, &live);
                let context = format!("{} with {total} replicas, live {live:?}", view.level);
                assert_eq!(
                    view.total_replicas as usize, requirement.total_replicas,
                    "{context}"
                );
                assert_eq!(
                    view.required_acks as usize, requirement.required_total,
                    "{context}"
                );
                assert_eq!(view.live_replicas as usize, live.len(), "{context}");
                assert_eq!(view.satisfiable, product.is_ok(), "{context}");
                assert_eq!(
                    view.reason,
                    product.err().map(|error| error.reason),
                    "{context}"
                );
            }
        }
    }
}
