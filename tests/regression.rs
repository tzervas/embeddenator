// Umbrella integration test crate for regression/backwards-compat suites.

#[path = "regression/e2e_regression.rs"]
mod e2e_regression;

#[path = "regression/compression_backward_compat.rs"]
mod compression_backward_compat;

#[path = "regression/compression_missing_codec.rs"]
mod compression_missing_codec;
