---
applyTo: '**/*.rs, Cargo.toml'
---

# 🦀 Rust Development & Parallel Execution Protocol

This protocol defines the mandatory operational standards for Rust development. It prioritizes memory safety, ownership-aware design, and the maximization of Cargo’s parallel execution capabilities.

## 🚨 CRITICAL: THE "ONE-MESSAGE" RULE

**1 MESSAGE = ALL MEMORY-SAFE OPERATIONS**
All Rust-related tasks must be executed concurrently and presented in a single, comprehensive response. Do not fragment code, testing, and optimization into separate steps.

### 🔴 Mandatory Post-Edit Workflow

Immediately following any code modification, the following sequence must be simulated or executed as a batch:

1. **`cargo fmt --all`**: Ensure strict adherence to style guidelines.
2. **`cargo clippy`**: Perform deep static analysis for idiomatic Rust.
3. **`cargo test --all`**: Execute the entire test suite in parallel.
4. **`cargo build`**: Final verification of compilation and dependency integrity.

## 🏗 Core Design Principles

| Principle | Requirement |
| --- | --- |
| **Ownership & Borrowing** | Enforce strict memory safety without a garbage collector. Minimize `unsafe` blocks. |
| **Concurrency** | Leverage `async/await`, `Tokio`, or `Rayon` for thread-safe, high-performance execution. |
| **Zero-Cost Abstractions** | Utilize traits and generics to ensure high-level code compiles to optimal machine code. |
| **Error Handling** | Mandatory use of `Result<T, E>` and `Option<T>`. Avoid `unwrap()` in production-ready code. |

## 🛠 Concurrent Execution Patterns

### 1. Cargo Operations

* **Parallel Commands**: Always batch `cargo build`, `cargo test`, and `cargo run` into single-call instructions.
* **Crate Management**: Install and update dependencies in batches to optimize lockfile generation.

### 2. Memory-Safe Coordination

* **Ownership Management**: Ensure all code snippets demonstrate correct lifetime annotations and borrowing patterns to prevent data races at compile time.
* **Async Implementation**: Use `Tokio` for I/O-bound tasks and `Rayon` for CPU-bound parallel iterators.

## 📚 Recommended Tech Stack & Tools

* **Runtime**: `Tokio` (Async), `Rayon` (Parallelism)
* **Web & API**: `Axum`, `Actix-web`, `Serde` (Serialization)
* **Database**: `SQLx`, `Diesel`
* **Utilities**: `Anyhow` (Error handling), `Clap` (CLI), `Itertools`
* **Analysis**: `cargo-expand`, `cargo-audit`, `cargo-flamegraph` (Profiling)

## 🎯 Final Goal: Production-Ready Output

The Agent must not simply provide "working" code. The output must be:

* **Formatted** (fmt)
* **Idiomatic** (clippy)
* **Verified** (test)
* **Optimized** (build)

**Remember:** Rust's power comes from its ability to be both safe and fast. Always leverage the compiler as your primary tool for ensuring parallel execution integrity.
