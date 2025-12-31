# ADR-007: Clippy Pedantic Linting

## Status

Accepted

## Date

2024-12-31

## Context

Code quality and consistency are critical for a long-lived project like an operating system. We need to establish linting standards that:
- Catch bugs early
- Enforce consistent style
- Encourage Rust best practices
- Document intent through explicit choices

## Decision

We will enable `clippy::pedantic` lint group workspace-wide with a minimal set of allowed exceptions, configured in `Cargo.toml`.

## Consequences

### Positive

- **Bug prevention**: Pedantic lints catch subtle issues before they become bugs
- **Consistent style**: Entire codebase follows same patterns
- **Better documentation**: Pedantic requires more explicit code, which serves as documentation
- **Learning tool**: New contributors learn Rust best practices from lint suggestions
- **Technical debt prevention**: Forces addressing issues immediately rather than accumulating debt
- **Safer unsafe**: Additional scrutiny on unsafe blocks

### Negative

- **Initial friction**: More work to satisfy strict lints when writing new code
- **False positives**: Some pedantic lints may not apply to our use cases
- **Verbosity**: May require more explicit type annotations and documentation
- **Kernel constraints**: Some lints don't apply well to no_std kernel code

### Neutral

- Will maintain an explicit allow list for lints that don't apply
- CI will enforce lint compliance
- Can add crate-level overrides where necessary

## Allowed Lint Exceptions

```rust
// Workspace-wide in Cargo.toml
#![allow(
    // These have legitimate use cases in our codebase
    clippy::module_name_repetitions,  // VortexConfig in vortex module is fine
    clippy::similar_names,            // kv_cache and kv_store are distinct concepts

    // Kernel-specific (only in kernel crate)
    clippy::missing_panics_doc,       // Kernel panics are by design
    clippy::missing_errors_doc,       // Will add progressively
)]
```

## Alternatives Considered

### Alternative 1: Default Clippy Only

Use only default Clippy warnings without pedantic.

**Pros:**
- Less friction for development
- Fewer false positives
- Faster iteration

**Cons:**
- Misses many valuable lints
- Inconsistent code quality
- Subtle bugs slip through

**Why not chosen:** The value of pedantic lints outweighs the additional effort, especially for a safety-critical project like an OS.

### Alternative 2: Custom Lint Selection

Hand-pick individual lints instead of using pedantic group.

**Pros:**
- Fine-grained control
- No unwanted lints

**Cons:**
- Maintenance burden (must evaluate each new lint)
- May miss valuable new lints added to pedantic
- Inconsistent with community standards

**Why not chosen:** Using the pedantic group and allowing exceptions is more maintainable and automatically benefits from upstream improvements.

### Alternative 3: Clippy Restriction Lints

Enable even stricter restriction lints.

**Pros:**
- Maximum strictness
- Catches even more edge cases

**Cons:**
- Many restriction lints are intentionally opinionated
- Significant false positive rate
- May require architectural changes to satisfy

**Why not chosen:** Restriction lints are too opinionated for a general policy. Can enable specific restriction lints case-by-case.

## Implementation

### Cargo.toml Configuration

```toml
[workspace.lints.rust]
unsafe_code = "warn"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
# Allowed exceptions
module_name_repetitions = "allow"
similar_names = "allow"
# Additional strict lints
unwrap_used = "warn"
expect_used = "warn"
```

### CI Enforcement

```yaml
- name: Clippy
  run: cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## References

- [Clippy Lint Categories](https://rust-lang.github.io/rust-clippy/master/index.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Configuration](https://doc.rust-lang.org/clippy/configuration.html)
