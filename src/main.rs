use std::string::ToString;

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
    #[arg(short, long, conflicts_with_all=["count"], help="Show all commits")]
    all: bool,
    #[arg(short, long, value_enum, conflicts_with_all=["count"], help="Show only commits with the given rarity")]
    only: Option<HashRarity>,
    #[arg(short, long, help = "Show commit count")]
    count: bool,
}

#[derive(Tabled, Display, Clone, PartialEq, Eq, ValueEnum)]
enum HashRarity {
    Common,
    Uncommon,
    Rare,
}

#[derive(Tabled)]
struct Count {
    #[tabled(rename = "Total")]
    total: usize,
    #[tabled(rename = "Common")]
    common: usize,
    #[tabled(rename = "Uncommon")]
    uncommon: usize,
    #[tabled(rename = "Rare")]
    rare: usize,
}

#[derive(Tabled, Clone)]
struct Commit {
    #[tabled(rename = "Author")]
    author: String,
    #[tabled(rename = "Datetime")]
    datetime: DateTime<FixedOffset>,
    #[tabled(rename = "Hash")]
    hash: String,
    #[tabled(rename = "Rarity")]
    rarity: HashRarity,
}

impl Commit {
    fn new(hash: String, author: String, datetime: DateTime<FixedOffset>) -> Self {
        Self {
            author,
            datetime,
            hash: String::from(&hash),
            rarity: Self::get_rarity(&hash),
        }
    }

    fn get_rarity(hash: &str) -> HashRarity {
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

fn print_table<T>(commits: &Vec<T>) -> Result<()>
where
    T: Tabled,
{
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
        println!("No commits found.");
        return Ok(());
    }
    let commits: Vec<Commit> = raw_output.par_lines().filter_map(parse_commit).collect();

    if args.all && args.only.is_none() {
        print_table(&commits)
    } else if let Some(only) = args.only {
        let only_commits = commits
            .par_iter()
            .filter(|c| c.rarity == only)
            .cloned()
            .collect::<Vec<Commit>>();
        if only_commits.is_empty() {
            println!("No {} commits found.", only);
            return Ok(());
        }
        return print_table(&only_commits);
    } else if args.count {
        let count = Count {
            total: commits.len(),
            common: commits
                .par_iter()
                .filter(|c| c.rarity == HashRarity::Common)
                .count(),
            uncommon: commits
                .par_iter()
                .filter(|c| c.rarity == HashRarity::Uncommon)
                .count(),
            rare: commits
                .par_iter()
                .filter(|c| c.rarity == HashRarity::Rare)
                .count(),
        };
        return print_table(&vec![count]);
    } else {
        let not_common_commits = commits
            .par_iter()
            .filter(|c| c.rarity != HashRarity::Common)
            .cloned()
            .collect::<Vec<Commit>>();

        if not_common_commits.is_empty() {
            println!("No uncommon or rare commits found.");
            return Ok(());
        }
        return print_table(&not_common_commits);
    }
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
