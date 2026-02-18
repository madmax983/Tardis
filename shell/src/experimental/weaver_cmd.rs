//! The Weaver Command
//!
//! "Weave" - Proactively find and suggest relationships between entities.

#![cfg(feature = "nova")]

use std::sync::Arc;
use tardis_chronos::experimental::weaver::Weaver;
use tardis_common::id::ModelHandle;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;

/// Run the Weaver command.
///
/// # Errors
///
/// Returns an error if the weaver fails.
pub async fn run(
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model_handle: Option<ModelHandle>,
    args: &[String],
) -> anyhow::Result<String> {
    let entity_name = if args.is_empty() {
        None
    } else {
        Some(args.join(" "))
    };

    println!("🕸️  The Weaver is scanning the timeline...");
    if let Some(name) = &entity_name {
        println!("   Focusing on: {}", name);
    } else {
        println!("   Scanning for disconnected threads...");
    }

    let weaver = Weaver::new(vortex, gallifrey.clone(), model_handle);
    let suggestions = weaver.weave(entity_name.as_deref()).await?;

    if suggestions.is_empty() {
        return Ok("No obvious connections found at this time.".to_string());
    }

    let mut report = String::new();
    report.push_str("Found potential connections:\n\n");

    for (i, suggestion) in suggestions.iter().enumerate() {
        report.push_str(&format!(
            "{}. {} --[{}]--> {}\n",
            i + 1,
            suggestion.source.name,
            suggestion.relationship_type,
            suggestion.target.name
        ));
        report.push_str(&format!("   📝 {}\n", suggestion.description));
        report.push_str(&format!("   📊 Confidence: {:.2}\n\n", suggestion.confidence));
    }

    // TODO: In a real interactive shell, we would ask the user to confirm insertion here.
    // For now, we just list them.
    // report.push_str("To confirm, run: `weave accept <index>` (Not implemented yet)");

    Ok(report)
}
