use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use strum_macros::Display;
use tabled::{settings::Style, Table, Tabled};
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[arg(short, long)]
    all: bool,
    #[arg(short, long, value_enum)]
    show: Option<Vec<HashRarity>>,
}

#[derive(Tabled, Display, Clone, ValueEnum)]
enum HashRarity {
    Common,
    Uncommon,
    Rare,
}

#[derive(Tabled)]
struct Commit {
    author: String,
    datetime: DateTime<FixedOffset>,
    hash: String,
    rarity: HashRarity,
}

impl Commit {
    fn new(hash: String, author: String, datetime: DateTime<FixedOffset>) -> Self {
        Self {
            author,
            datetime,
            hash: String::from(&hash),
            rarity: Self::calculate_rarity(&hash),
        }
    }

    fn calculate_rarity(hash: &str) -> HashRarity {
        if Self::is_rare(hash) {
            HashRarity::Rare
        } else if Self::is_uncommon(hash) {
            HashRarity::Uncommon
        } else {
            HashRarity::Common
        }
    }

    fn is_uncommon(hash: &str) -> bool {
        hash.chars().take(9).all(|c| c.is_ascii_digit())
    }

    fn is_rare(hash: &str) -> bool {
        hash.chars().take(9).all(|c| !c.is_ascii_digit())
    }
}

fn parse_commit(line: &str) -> Option<Commit> {
    let mut parts = line.split_whitespace();
    let hash = parts.next();
    let datetime = parts.next();
    let author = parts.collect::<Vec<&str>>().join(" ");
    match (hash, author, datetime) {
        (Some(hash), author, Some(datetime)) => {
            let datetime = DateTime::parse_from_rfc3339(datetime).ok()?;
            Some(Commit::new(hash.to_string(), author.to_string(), datetime))
        }
        _ => None,
    }
}

fn print_all_table(commits: &Vec<Commit>) -> Result<()> {
    let mut table = Table::new(commits);
    table.with(Style::rounded());
    println!("{table}");
    Ok(())
}

fn main() -> Result<()> {
    let args = CliArgs::parse();
    let sh = Shell::new()?;

    let raw_output = cmd!(sh, "git log --pretty=format:'%H %aI %an'").read()?;
    if raw_output.is_empty() {
        println!("No commits found");
        return Ok(());
    }
    let commits: Vec<Commit> = raw_output.par_lines().filter_map(parse_commit).collect();

    if args.all {
        return print_all_table(&commits);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::*;
    #[test]
    fn verify_app() {
        use clap::CommandFactory;
        CliArgs::command().debug_assert()
    }
}
