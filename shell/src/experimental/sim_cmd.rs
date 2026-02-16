//! `sim` Command Handler.
//!
//! Subcommands for managing timelines.

use super::simulacrum::Simulacrum;
use anyhow::{anyhow, Result};
use std::fmt::Write;

/// Run the `sim` command.
///
/// # Errors
///
/// Returns an error if the command fails.
pub fn run(simulacrum: &mut Simulacrum, args: &[String]) -> Result<String> {
    if args.is_empty() {
        return Ok("Usage: sim <command> [args...]".to_string());
    }

    let command = &args[0];
    let sub_args = &args[1..];

    match command.as_str() {
        "fork" => {
            if sub_args.is_empty() {
                return Err(anyhow!("Usage: sim fork <name>"));
            }
            let name = &sub_args[0];
            simulacrum.fork(name)?;
            Ok(format!("Created timeline '{name}'"))
        }
        "ls" | "list" => {
            let names = simulacrum.list();
            if names.is_empty() {
                Ok("No active timelines.".to_string())
            } else {
                let mut output = String::new();
                writeln!(output, "Active Timelines:")?;
                for name in names {
                    writeln!(output, "  - {name}")?;
                }
                Ok(output)
            }
        }
        "rm" | "remove" => {
            if sub_args.is_empty() {
                return Err(anyhow!("Usage: sim rm <name>"));
            }
            let name = &sub_args[0];
            simulacrum.remove(name)?;
            Ok(format!("Removed timeline '{name}'"))
        }
        "diff" => {
            if sub_args.is_empty() {
                return Err(anyhow!("Usage: sim diff <name>"));
            }
            let name = &sub_args[0];
            let timeline = simulacrum
                .get(name)
                .ok_or_else(|| anyhow!("Timeline '{name}' not found"))?;

            let (modified, deleted) = timeline
                .list_changes()
                .map_err(|e| anyhow!(e.to_string()))?;

            if modified.is_empty() && deleted.is_empty() {
                return Ok(format!("Timeline '{name}' is identical to base reality."));
            }

            let mut output = String::new();
            writeln!(output, "Delta Report: {name}")?;
            writeln!(output, "======================")?;

            if !modified.is_empty() {
                writeln!(output, "Modified/Added Entities:")?;
                for entity in modified {
                    writeln!(output, "  + {} ({})", entity.name, entity.entity_type)?;
                }
            }

            if !deleted.is_empty() {
                writeln!(output, "Deleted Entities:")?;
                for id in deleted {
                    writeln!(output, "  - {id}")?;
                }
            }

            Ok(output)
        }
        "inspect" => {
            if sub_args.len() < 2 {
                return Err(anyhow!("Usage: sim inspect <name> <entity_id>"));
            }
            let name = &sub_args[0];
            let entity_id_str = &sub_args[1];

            // We need to parse EntityId. It's Uuid based usually.
            // But wait, user might provide name?
            // "inspect <name> <entity_name>" is nicer but getting ID by name requires scanning.
            // Timeline::scan_history can be used.

            let timeline = simulacrum
                .get(name)
                .ok_or_else(|| anyhow!("Timeline '{name}' not found"))?;

            let mut found = None;

            // Find entity by name or ID
            timeline
                .scan_history(|history| {
                    if found.is_some() {
                        return;
                    }

                    // Check if any version matches name or ID
                    if let Some(current) = history.iter().find(|e| e.temporal.is_current()) {
                        if current.name == *entity_id_str
                            || current.id.to_string() == *entity_id_str
                        {
                            found = Some(current.clone());
                        }
                    }
                })
                .map_err(|e| anyhow!(e.to_string()))?;

            if let Some(entity) = found {
                let props = serde_json::to_string_pretty(&entity.properties)
                    .unwrap_or_else(|_| format!("{:?}", entity.properties));
                Ok(format!(
                    "Entity: {} ({})\nID: {}\nProperties:\n{}",
                    entity.name, entity.entity_type, entity.id, props
                ))
            } else {
                Err(anyhow!(
                    "Entity '{entity_id_str}' not found in timeline '{name}'"
                ))
            }
        }
        _ => Err(anyhow!("Unknown subcommand: {command}")),
    }
}
