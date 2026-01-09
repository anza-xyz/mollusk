//! Compute unit benchmarking results and checks.

use {
    mollusk_svm::result::InstructionResult,
    num_format::{Locale, ToFormattedString},
    std::path::Path,
};

pub struct MolluskComputeUnitBenchResult<'a> {
    name: &'a str,
    cus_consumed: u64,
}

impl<'a> MolluskComputeUnitBenchResult<'a> {
    pub fn new(name: &'a str, result: InstructionResult) -> Self {
        let cus_consumed = result.compute_units_consumed;
        Self { name, cus_consumed }
    }
}

pub struct MolluskComputeUnitMatrixBenchResult<'a> {
    program_name: &'a str,
    ix_name: &'a str,
    cus_consumed: u64,
}

impl<'a> MolluskComputeUnitMatrixBenchResult<'a> {
    pub fn new(program_name: &'a str, ix_name: &'a &str, result: InstructionResult) -> Self {
        let cus_consumed = result.compute_units_consumed;
        Self {
            program_name,
            ix_name,
            cus_consumed,
        }
    }
}

pub fn write_results(
    out_dir: &Path,
    table_header: &str,
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
    let mut md_table = md_header(table_header, solana_version);

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

fn md_header(table_header: &str, solana_version: &str) -> String {
    format!(
        r#"#### {}

Solana CLI Version: {}

| Name | CUs | Delta |
|------|------|-------|
"#,
        table_header, solana_version,
    )
}

fn parse_last_md_table(content: &str) -> Vec<MolluskComputeUnitBenchResult<'_>> {
    let mut results = vec![];

    for line in content.lines().skip(6) {
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

pub fn mx_write_results(
    out_dir: &Path,
    table_header: &str,
    solana_version: &str,
    results: Vec<MolluskComputeUnitMatrixBenchResult>,
) {
    if results.is_empty() {
        return;
    }

    let mut mx_md_table = mx_md_header(table_header, solana_version, &results);
    let mut current_ix = "";

    for result in &results {
        if result.ix_name != current_ix {
            // If we are moving from one instruction to the next, close the previous line
            if !current_ix.is_empty() {
                mx_md_table.push_str("|\n");
            }
            // Start the new row with the instruction name
            mx_md_table.push_str(&format!("| `{}` ", result.ix_name));
            current_ix = result.ix_name;
        }

        // Add the CU value for this program in the current row
        mx_md_table.push_str(&format!("| {} ", result.cus_consumed));
    }

    // Close the final row
    mx_md_table.push_str("|\n");

    let path = out_dir.join("mx_compute_units.md");
    prepend_to_md_file(&path, &mx_md_table);
}

fn mx_md_header(
    table_header: &str,
    solana_version: &str,
    results: &[MolluskComputeUnitMatrixBenchResult],
) -> String {
    //Get ONLY the unique program names.
    let mut program_names = Vec::new();
    for r in results {
        if !program_names.contains(&r.program_name) {
            program_names.push(r.program_name);
        }
    }

    // Header: | Name | CU (p1) | CU (p2) | ...
    let mut header_row = String::from("| Name ");
    for program_name in &program_names {
        header_row.push_str(&format!("| CU (`{}`) ", program_name));
    }
    header_row.push('|');

    // Separator: |----------|----------|----------| ...
    let separator = std::iter::repeat("|----------")
        .take(program_names.len() + 1)
        .collect::<String>()
        + "|";

    format!(
        "#### {}

Solana CLI Version: {}

{}
{}\n",
        table_header, solana_version, header_row, separator
    )
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
