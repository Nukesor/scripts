use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};

pub fn pretty_table() -> Table {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.load_preset(UTF8_FULL_CONDENSED);

    table
}

pub fn print_headline_table(header: String) {
    let mut table = pretty_table();
    table.add_row(vec![header]);

    println!("{table}");
}
