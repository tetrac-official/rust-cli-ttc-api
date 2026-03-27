use colored::Colorize;
use serde::Serialize;

use crate::models::{Balance, Order, Position, Ticker};
use super::OutputFormat;

/// Printer helper struct for consistent output formatting
pub struct Printer {
    format: OutputFormat,
}

impl Printer {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn success(&self, msg: &str) {
        println!("{} {}", "OK".green(), msg);
    }

    pub fn warning(&self, msg: &str) {
        println!("{} {}", "WARN".yellow(), msg);
    }

    pub fn info(&self, msg: &str) {
        println!("{} {}", "INFO".blue(), msg);
    }

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
        let pnl_color = if self.unrealized_pnl >= 0.0 {
            format!("+${:.2}", self.unrealized_pnl).green()
        } else {
            format!("-${:.2}", self.unrealized_pnl.abs()).red()
        };

        println!(
            "{} {} {} {}x @ ${:.2} | PnL: {}",
            "POS".blue(),
            self.symbol.white(),
            self.position_side.to_uppercase().yellow(),
            self.leverage,
            self.entry_price,
            pnl_color
        );
        println!(
            "   Size: {} | Mark: ${:.2} | Liq: ${:.2}",
            self.size, self.mark_price, self.liquidation_price
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
            self.unrealized_pnl,
            self.leverage,
            self.liquidation_price,
            self.margin_type,
            self.notional
        );
    }

    fn print_quiet(&self) {
        println!("{}:{}", self.symbol, self.position_side);
    }
}

impl Tableable for Balance {
    fn print_table(&self) {
        let locked_str = if self.locked > 0.0 {
            format!(" ({} locked)", self.locked).yellow().to_string()
        } else {
            String::new()
        };

        println!(
            "{} {}: {} {}",
            "BAL".yellow(),
            self.asset.white().bold(),
            format!("{:.4}", self.balance).bright_green(),
            locked_str
        );
        println!("   Available: {}", format!("{:.4}", self.available).bright_white());
    }

    fn print_csv_header() {
        println!("asset,balance,available,locked");
    }

    fn print_csv(&self) {
        println!("{},{},{},{}", self.asset, self.balance, self.available, self.locked);
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
            "{} {} | ${:.2} | {}",
            "MKT".purple(),
            self.symbol.white().bold(),
            self.last_price,
            change_color
        );
        println!(
            "   Vol: ${:.0} | High: ${:.2} | Low: ${:.2}",
            self.volume, self.high, self.low
        );
    }

    fn print_csv_header() {
        println!("symbol,last_price,price_change_percent,volume,high,low");
    }

    fn print_csv(&self) {
        println!(
            "{},{},{},{},{},{}",
            self.symbol, self.last_price, self.price_change_percent, self.volume, self.high, self.low
        );
    }

    fn print_quiet(&self) {
        println!("{}:{}", self.symbol, self.last_price);
    }
}
