// Umbrella integration test crate for invariant/correctness suites.
//
// Keeping this as a top-level file ensures Cargo discovers and runs the suite,
// while the actual tests live in subdirectories for navigability.

#[path = "invariants/bitsliced_equivalence.rs"]
mod bitsliced_equivalence;

#[path = "invariants/bt_phase1_packed_equivalence.rs"]
mod bt_phase1_packed_equivalence;

#[path = "invariants/bt_phase2_scratch_invariants.rs"]
mod bt_phase2_scratch_invariants;

#[path = "invariants/packed_trit_vec.rs"]
mod packed_trit_vec;

#[path = "invariants/ternary_refactor_invariants.rs"]
mod ternary_refactor_invariants;

#[path = "invariants/exhaustive_trit_tests.rs"]
mod exhaustive_trit_tests;

#[path = "invariants/reconstruction_guarantee.rs"]
mod reconstruction_guarantee;

#[path = "invariants/ternary_signature_index.rs"]
mod ternary_signature_index;

#[path = "invariants/envelope_edge_cases.rs"]
mod envelope_edge_cases;

#[path = "invariants/extended_dimensionality.rs"]
mod extended_dimensionality;

#[path = "invariants/simd_cosine_tests.rs"]
mod simd_cosine_tests;

#[path = "invariants/lens_contract.rs"]
mod lens_contract;

#[path = "invariants/lens_variants.rs"]
mod lens_variants;

#[cfg(feature = "proptest")]
#[path = "invariants/register_validity.rs"]
mod register_validity;

#[cfg(feature = "proptest")]
#[path = "invariants/block_sparse_invariants.rs"]
mod block_sparse_invariants;
