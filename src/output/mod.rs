mod printer;

pub use printer::Printer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
    Quiet,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Quiet => write!(f, "quiet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_table() {
        assert_eq!(OutputFormat::default(), OutputFormat::Table);
    }

    #[test]
    fn display_matches_clap_lowercase_token() {
        // Each lowercase token here is what users pass to --output-format.
        // If the Display impl drifts, the help text and the actual flag
        // accept the same string by accident; this test forces them aligned.
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Csv.to_string(), "csv");
        assert_eq!(OutputFormat::Quiet.to_string(), "quiet");
    }

    #[test]
    fn equality_and_copy_semantics() {
        // OutputFormat is Copy + Eq — used in `if format != OutputFormat::Quiet`
        // checks throughout printer.rs. Lock in those traits.
        let a = OutputFormat::Json;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(OutputFormat::Json, OutputFormat::Csv);
    }
}
