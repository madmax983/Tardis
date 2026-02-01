# Sentry's Journal

**[Tracing Macro Argument Evaluation]**
**Learning:** Rust's `tracing` macros (like `info!`, `debug!`) may skip argument evaluation if the log level is disabled or no subscriber is initialized. This can mask panics that occur within the arguments (e.g., `info!("{}", &s[..50])` where the slice panics).
**Action:** When testing code that uses logging macros, ensuring the arguments are evaluated, or testing the logic outside the macro, is crucial. Do not rely on logging statements to trigger side effects or panics in tests unless logging is explicitly enabled.
