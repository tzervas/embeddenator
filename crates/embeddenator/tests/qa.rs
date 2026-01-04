// Umbrella integration test crate for QA-focused suites.

#[path = "qa/common_smoke.rs"]
mod common_smoke;

#[path = "qa/unit_tests.rs"]
mod unit_tests;

#[path = "qa/test_metrics_integrity.rs"]
mod test_metrics_integrity;

#[path = "qa/qa_comprehensive.rs"]
mod qa_comprehensive;
