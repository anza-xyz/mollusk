//! Compute unit benchmarking results and checks.

use {
    chrono::{DateTime, Utc},
    mollusk_svm::result::InstructionResult,
    num_format::{Locale, ToFormattedString},
    std::path::Path,
};

pub(crate) struct MolluskComputeUnitBenchResult<'a> {
    name: &'a str,
    cus_consumed: u64,
}

impl<'a> MolluskComputeUnitBenchResult<'a> {
    pub fn new(name: &'a str, result: InstructionResult) -> Self {
        let cus_consumed = result.compute_units_consumed;
        Self { name, cus_consumed }
    }
}

pub(crate) fn write_results(
    out_dir: &Path,
    solana_version: &str,
    results: Vec<MolluskComputeUnitBenchResult>,
) {
    let path = out_dir.join("compute_units.md");

    // Load the existing bench content and parse the most recent table.
    let mut no_changes = true;
    let existing_content = if path.exists() {
        Some(std::fs::read_to_string(&path).unwrap())
    } else {
        None
    };
    let previous = existing_content
        .as_ref()
        .map(|content| parse_last_md_table(content));

    // Prepare to write a new table.
    let mut md_table = md_header(solana_version);

    // Evaluate the results against the previous table, if any.
    // If there are changes, write a new table.
    // If there are no changes, break out and abort gracefully.
    for result in results {
        let delta = match previous.as_ref().and_then(|prev_results| {
            prev_results
                .iter()
                .find(|prev_result| prev_result.name == result.name)
        }) {
            Some(prev) => {
                let delta = result.cus_consumed as i64 - prev.cus_consumed as i64;
                if delta == 0 {
                    "--".to_string()
                } else {
                    no_changes = false;
                    if delta > 0 {
                        format!("+{}", delta.to_formatted_string(&Locale::en))
                    } else {
                        delta.to_formatted_string(&Locale::en)
                    }
                }
            }
            None => {
                no_changes = false;
                "- new -".to_string()
            }
        };
        md_table.push_str(&format!(
            "| {} | {} | {} |\n",
            result.name, result.cus_consumed, delta
        ));
    }

    // Only create a new table if there were changes.
    if !no_changes {
        md_table.push('\n');
        prepend_to_md_file(&path, &md_table);
    }
}

fn md_header(solana_version: &str) -> String {
    let now: DateTime<Utc> = Utc::now();
    format!(
        r#"#### Compute Units: {}

Solana CLI Version: {}

| Name | CUs | Delta |
|------|------|-------|
"#,
        now, solana_version,
    )
}

fn parse_last_md_table(content: &str) -> Vec<MolluskComputeUnitBenchResult> {
    let mut results = vec![];

    for line in content.lines().skip(4) {
        if line.starts_with("####") || line.is_empty() {
            break;
        }

        let mut parts = line.split('|').skip(1).map(str::trim);
        let name = parts.next().unwrap();
        let cus_consumed = parts.next().unwrap().parse().unwrap();

        results.push(MolluskComputeUnitBenchResult { name, cus_consumed });
    }

    results
}

fn prepend_to_md_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let contents = if path.exists() {
        std::fs::read_to_string(path).unwrap()
    } else {
        String::new()
    };

    let mut new_contents = content.to_string();
    new_contents.push_str(&contents);

    std::fs::write(path, new_contents).unwrap();
}
