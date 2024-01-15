use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use clap::Parser;
use strum_macros::Display;
use tabled::{settings::Style, Table, Tabled};
use xshell::{cmd, Shell};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[arg(short, long)]
    all: bool,
    #[arg(short, long)]
    top: bool,
}

#[derive(Tabled, Display)]
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

fn group_commits(log: &str) -> Vec<&str> {
    let mut commits = vec![];
    let mut start = 0;

    for (idx, line) in log.lines().enumerate() {
        if line.starts_with("commit") && idx != 0 {
            commits.push(&log[start..idx]);
            start = idx;
        }
    }
    commits.push(&log[start..]);

    commits
}

fn parse_commit(commit: &&str) -> Option<Commit> {
    let lines: Vec<&str> = commit.lines().collect();
    let hash = lines.get(0).and_then(|line| line.split_whitespace().nth(1));
    let author = lines.get(1).and_then(|line| line.split_whitespace().nth(1));
    let datetime = lines.get(2).and_then(|line| line.split_whitespace().nth(1));
    let parsed_datetime = match datetime {
        Some(datetime) => DateTime::parse_from_rfc3339(datetime).ok(),
        None => None,
    };
    match (hash, author, parsed_datetime) {
        (Some(hash), Some(author), Some(datetime)) => {
            Some(Commit::new(hash.to_string(), author.to_string(), datetime))
        }
        _ => None,
    }
}

fn main() -> Result<()> {
    let args = CliArgs::parse();
    let sh = Shell::new()?;

    let raw_output = cmd!(sh, "git log --pretty=medium --date=iso-strict").read()?;
    if raw_output.is_empty() {
        println!("No commits found");
        return Ok(());
    }
    let grouped_commits = group_commits(&raw_output);
    let commits: Vec<Commit> = grouped_commits.iter().filter_map(parse_commit).collect();

    let mut table = Table::new(commits);
    table.with(Style::empty());

    println!("{table}");
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
