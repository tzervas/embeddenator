// Umbrella integration test crate for hierarchical format + unfolding suites.

#[path = "hierarchical/hierarchical_artifacts_e2e.rs"]
mod hierarchical_artifacts_e2e;

#[path = "hierarchical/hierarchical_determinism.rs"]
mod hierarchical_determinism;

#[path = "hierarchical/hierarchical_unfolding.rs"]
mod hierarchical_unfolding;
