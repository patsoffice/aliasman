use comfy_table::{ContentArrangement, Table};

use aliasman_core::model::Alias;

pub fn print_alias_table(aliases: &[Alias]) {
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "#",
            "Alias",
            "Email Address(es)",
            "Description",
            "Suspended",
            "Created",
            "Modified",
            "Suspended At",
        ]);

    for (i, alias) in aliases.iter().enumerate() {
        let suspended_at = alias
            .suspended_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();

        table.add_row(vec![
            (i + 1).to_string(),
            alias.full_alias(),
            alias.email_addresses.join(", "),
            alias.description.clone(),
            if alias.suspended { "Yes" } else { "No" }.to_string(),
            alias.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            alias.modified_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            suspended_at,
        ]);
    }

    println!("{table}");
}
