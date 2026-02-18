//! Shell command for The Weaver.
//!
//! Usage: `weave <source> to <target>` or `weave <source>, <target>`

use anyhow::{Context, Result};
use std::fmt::Write;
use std::sync::Arc;
use tardis_chronos::experimental::weaver::Weaver;
use tardis_chronos::Chronos;
use tardis_gallifrey::Gallifrey;

/// Run the `weave` command.
///
/// # Errors
///
/// Returns an error if the connection fails or arguments are invalid.
pub async fn run(
    gallifrey: Arc<Gallifrey>,
    chronos: Arc<Chronos>,
    args: &[String],
) -> Result<String> {
    if args.is_empty() {
        return Ok("Usage: weave <source> to <target> OR weave <source>, <target>".to_string());
    }

    let full_args = args.join(" ");
    let (source, target) = if full_args.contains(" to ") {
        let parts: Vec<&str> = full_args.splitn(2, " to ").collect();
        (parts[0].trim().to_string(), parts[1].trim().to_string())
    } else if full_args.contains(',') {
        let parts: Vec<&str> = full_args.splitn(2, ',').collect();
        (parts[0].trim().to_string(), parts[1].trim().to_string())
    } else if args.len() == 2 {
        (args[0].clone(), args[1].clone())
    } else {
        return Ok("Usage: weave <source> to <target> OR weave <source>, <target>".to_string());
    };

    let vortex = chronos.vortex();
    let loaded_models = vortex.list_loaded_models();

    // Check if we have a model, otherwise hint to load one
    let (model, _) = loaded_models
        .first()
        .context("No models loaded. Use 'models load <path>' first.")?;

    let weaver = Weaver::new(Arc::clone(&vortex), Arc::clone(&gallifrey));

    println!("🧶 Weaving connection between '{source}' and '{target}'...");

    let result = weaver.weave(&source, &target, *model).await?;

    let mut output = String::new();
    let _ = write!(
        output,
        "\n✨ THE CONNECTION ✨\n\n{}\n\n",
        result.narrative
    );
    output.push_str("📜 THE PATH:\n");

    for (i, step) in result.steps.iter().enumerate() {
        let _ = writeln!(output, "{}. {}", i + 1, step.description);
        if let Some(entity) = &step.entity_name {
            let _ = writeln!(output, "   (Entity: {entity})");
        }
        if let Some(rel) = &step.relationship {
            let _ = writeln!(output, "   (Relation: {rel})");
        }
    }

    Ok(output)
}
