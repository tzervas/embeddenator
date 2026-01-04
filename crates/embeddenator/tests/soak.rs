// Umbrella integration test crate for memory/scale/soak suites.

#[path = "soak/memory_scaled.rs"]
mod memory_scaled;

#[path = "soak/soak_memory.rs"]
mod soak_memory;
