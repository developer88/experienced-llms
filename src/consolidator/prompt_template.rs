use std::fs;
use std::path::Path;

use crate::config;

pub use crate::config::DEFAULT_CONSOLIDATION_TEMPLATE;

pub fn get_consolidation_template(
    home_dir: Option<&Path>,
) -> Result<String, Box<dyn std::error::Error>> {
    let home = home_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(config::get_experienced_home);

    let default_template = home.join("template.md");
    if default_template.exists() {
        return Ok(fs::read_to_string(&default_template)?);
    }

    let legacy_template = home.join("consolidation_prompt.txt");
    if legacy_template.exists() {
        return Ok(fs::read_to_string(&legacy_template)?);
    }

    Err(format!(
        "Consolidation template missing at '{}'. Run 'experienced-llms init' to generate the default template.",
        default_template.display()
    ).into())
}
