use colored::Colorize;
use serde::Serialize;

use super::OutputFormat;
use crate::models::{Balance, Order, Position, Ticker};

/// Printer helper struct for consistent output formatting
pub struct Printer {
    format: OutputFormat,
}

impl Printer {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    // Status messages always go to stderr so stdout stays pristine for
    // structured output (JSON/CSV) consumed by agents. In Table mode a human
    // reading the terminal still sees both streams intermixed.
    pub fn success(&self, msg: &str) {
        eprintln!("{} {}", "OK".green(), msg);
    }

    pub fn warning(&self, msg: &str) {
        eprintln!("{} {}", "WARN".yellow(), msg);
    }

    pub fn info(&self, msg: &str) {
        eprintln!("{} {}", "INFO".blue(), msg);
    }

    // dry_run is the actual result of a dry-run command — keep it on stdout
    // so it can be captured/piped like any other output.
    pub fn dry_run(&self, msg: &str) {
        println!("{} {}", "DRY-RUN".cyan(), msg.yellow());
    }

    pub fn print<T: Serialize + Tableable>(&self, data: &T) {
        match self.format {
            OutputFormat::Json => print_json(data),
            OutputFormat::Table => data.print_table(),
            OutputFormat::Csv => {
                T::print_csv_header();
                data.print_csv();
            }
            OutputFormat::Quiet => data.print_quiet(),
        }
    }

    pub fn print_list<T: Serialize + Tableable>(&self, items: &[T]) {
        if items.is_empty() {
            if self.format != OutputFormat::Quiet {
                println!("{}", "No results found".yellow());
            }
            return;
        }

        match self.format {
            OutputFormat::Json => print_json(items),
            OutputFormat::Table => items.iter().for_each(|item| item.print_table()),
            OutputFormat::Csv => {
                T::print_csv_header();
                items.iter().for_each(|item| item.print_csv());
            }
            OutputFormat::Quiet => items.iter().for_each(|item| item.print_quiet()),
        }
    }
}

fn print_json<T: Serialize + ?Sized>(data: &T) {
    match serde_json::to_string_pretty(data) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize to JSON: {}", e),
    }
}

pub trait Tableable: Serialize {
    fn print_table(&self);
    fn print_csv_header();
    fn print_csv(&self);
    fn print_quiet(&self);
}

impl Tableable for Order {
    fn print_table(&self) {
        let status = match self.status.as_str() {
            "new" => self.status.green(),
            "filled" => self.status.bright_green(),
            "canceled" => self.status.yellow(),
            "rejected" => self.status.red(),
            _ => self.status.normal(),
        };

        println!(
            "{} Order {} {} {} {} @ {}",
            "OK".green(),
            self.order_id.to_string().cyan(),
            self.symbol.white(),
            self.side.to_uppercase().yellow(),
            self.quantity,
            format!("{}", self.price).bright_white(),
        );
        println!("   Status: {} | Type: {}", status, self.order_type);
    }

    fn print_csv_header() {
        println!("order_id,symbol,side,position_side,order_type,quantity,price,status");
    }

    fn print_csv(&self) {
        println!(
            "{},{},{},{},{},{},{},{}",
            self.order_id,
            self.symbol,
            self.side,
            self.position_side,
            self.order_type,
            self.quantity,
            self.price,
            self.status
        );
    }

    fn print_quiet(&self) {
        println!("{}", self.order_id);
    }
}

impl Tableable for Position {
    fn print_table(&self) {
        let pnl_val = self.effective_pnl();
        let pnl_color = if pnl_val >= 0.0 {
            format!("+${:.2}", pnl_val).green()
        } else {
            format!("-${:.2}", pnl_val.abs()).red()
        };

        println!(
            "{} {} {} {}x @ ${:.4} | PnL: {}",
            "POS".blue(),
            self.symbol.white(),
            self.position_side.to_uppercase().yellow(),
            self.leverage,
            self.entry_price,
            pnl_color
        );
        let liq_display = match self.liquidation_price {
            Some(v) => format!("${:.4}", v),
            None => "n/a".to_string(),
        };
        println!(
            "   Size: {} | Mark: ${:.4} | Liq: {}",
            self.size, self.mark_price, liq_display
        );
    }

    fn print_csv_header() {
        println!("symbol,position_side,size,entry_price,mark_price,unrealized_pnl,leverage,liquidation_price,margin_type,notional");
    }

    fn print_csv(&self) {
        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            self.symbol,
            self.position_side,
            self.size,
            self.entry_price,
            self.mark_price,
            self.effective_pnl(),
            self.leverage,
            self.liquidation_price.unwrap_or(0.0),
            self.margin_type.as_deref().unwrap_or(""),
            self.notional.unwrap_or(0.0)
        );
    }

    fn print_quiet(&self) {
        println!("{}:{}", self.symbol, self.position_side);
    }
}

impl Tableable for Balance {
    fn print_table(&self) {
        let locked_str = match self.locked {
            Some(v) if v > 0.0 => format!(" ({} locked)", v).yellow().to_string(),
            _ => String::new(),
        };

        println!(
            "{} {}: {} {}",
            "BAL".yellow(),
            self.asset.white().bold(),
            format!("{:.4}", self.balance).bright_green(),
            locked_str
        );
        println!(
            "   Available: {}",
            format!("{:.4}", self.available).bright_white()
        );
    }

    fn print_csv_header() {
        println!("asset,balance,available,locked");
    }

    fn print_csv(&self) {
        println!(
            "{},{},{},{}",
            self.asset,
            self.balance,
            self.available,
            self.locked.unwrap_or(0.0)
        );
    }

    fn print_quiet(&self) {
        println!("{}:{}", self.asset, self.balance);
    }
}

impl Tableable for Ticker {
    fn print_table(&self) {
        let change_color = if self.price_change_percent >= 0.0 {
            format!("+{:.2}%", self.price_change_percent).green()
        } else {
            format!("{:.2}%", self.price_change_percent).red()
        };

        println!(
            "{} {} | ${:.4} | {}",
            "MKT".purple(),
            self.symbol.white().bold(),
            self.last_price,
            change_color
        );
        println!(
            "   Vol: ${:.0} | High: ${:.4} | Low: ${:.4}",
            self.volume, self.high, self.low
        );
    }

    fn print_csv_header() {
        println!("symbol,last_price,price_change_percent,volume,high,low");
    }

    fn print_csv(&self) {
        println!(
            "{},{},{},{},{},{}",
            self.symbol,
            self.last_price,
            self.price_change_percent,
            self.volume,
            self.high,
            self.low
        );
    }

    fn print_quiet(&self) {
        println!("{}:{}", self.symbol, self.last_price);
    }
}
