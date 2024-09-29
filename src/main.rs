use std::fmt::{Display, Error, Formatter};
use std::string::ToString;
use std::time::Instant;

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
    only: Option<RarityTier>,
    #[arg(short, long, help = "Show commit count")]
    count: bool,
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

#[derive(Tabled, Clone, PartialEq)]
struct Rarity {
    #[tabled(rename = "Explanation")]
    explanation: String,
    #[tabled(rename = "Percentage")]
    percentage: f64,
    #[tabled(rename = "Tier")]
    tier: RarityTier,
}

#[derive(Tabled, Display, Clone, PartialEq, ValueEnum)]
enum RarityTier {
    Common,
    Uncommon,
    Rare,
}

#[derive(Tabled, Clone)]
struct Commit {
    #[tabled(rename = "Author")]
    author: String,
    #[tabled(rename = "Datetime")]
    datetime: DateTime<FixedOffset>,
    #[tabled(rename = "Hash")]
    hash: String,
    #[tabled(inline)]
    rarity: Rarity,
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

    fn get_rarity(hash: &str) -> Rarity {
        match hash {
            _ if UncommonExpl::is_starts_nine_digits(hash) => Rarity {
                tier: RarityTier::Uncommon,
                percentage: 0.01,
                explanation: UncommonExpl::StartsNineDigits.to_string(),
            },
            _ if UncommonExpl::is_ends_nine_digits(hash) => Rarity {
                tier: RarityTier::Uncommon,
                percentage: 0.01,
                explanation: UncommonExpl::EndsNineDigits.to_string(),
            },
            _ if UncommonExpl::is_contains_nine_continuous_digits(hash) => Rarity {
                tier: RarityTier::Uncommon,
                percentage: 0.01,
                explanation: UncommonExpl::ContainsNineContDigits.to_string(),
            },
            _ if RareExpl::is_starts_nine_letters(hash) => Rarity {
                tier: RarityTier::Rare,
                percentage: 0.001,
                explanation: RareExpl::StartsNineLetters.to_string(),
            },
            _ if RareExpl::is_ends_nine_letters(hash) => Rarity {
                tier: RarityTier::Rare,
                percentage: 0.001,
                explanation: RareExpl::EndsNineLetters.to_string(),
            },
            _ if RareExpl::is_contains_nine_continuous_letters(hash) => Rarity {
                tier: RarityTier::Rare,
                percentage: 0.001,
                explanation: RareExpl::ContainsNineContLetters.to_string(),
            },
            _ => Rarity {
                tier: RarityTier::Common,
                percentage: 0.99,
                explanation: String::from(""),
            },
        }
    }
}

enum UncommonExpl {
    StartsNineDigits,
    EndsNineDigits,
    ContainsNineContDigits,
}

impl Display for UncommonExpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Self::StartsNineDigits => write!(f, "Starts with nine digits"),
            Self::EndsNineDigits => write!(f, "Ends with nine digits"),
            Self::ContainsNineContDigits => write!(f, "Contains nine continuous digits"),
        }
    }
}

impl UncommonExpl {
    fn is_starts_nine_digits(hash: &str) -> bool {
        hash.chars().take(9).all(|c| c.is_ascii_digit())
    }

    fn is_ends_nine_digits(hash: &str) -> bool {
        hash.chars().rev().take(9).all(|c| c.is_ascii_digit())
    }

    fn is_contains_nine_continuous_digits(hash: &str) -> bool {
        hash.chars().collect::<String>().contains("999999999")
    }
}

enum RareExpl {
    StartsNineLetters,
    EndsNineLetters,
    ContainsNineContLetters,
}

impl Display for RareExpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Self::StartsNineLetters => write!(f, "Starts with nine letters"),
            Self::EndsNineLetters => write!(f, "Ends with nine letters"),
            Self::ContainsNineContLetters => write!(f, "Contains nine continuous letters"),
        }
    }
}

impl RareExpl {
    fn is_starts_nine_letters(hash: &str) -> bool {
        hash.chars().take(9).all(|c| c.is_ascii_alphabetic())
    }
    fn is_ends_nine_letters(hash: &str) -> bool {
        hash.chars().rev().take(9).all(|c| c.is_ascii_alphabetic())
    }
    fn is_contains_nine_continuous_letters(hash: &str) -> bool {
        hash.chars().collect::<String>().contains("abcdefghi")
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

fn print_table<T>(commits: &Vec<T>, start_time: Instant) -> Result<()>
where
    T: Tabled,
{
    let mut table = Table::new(commits);
    table.with(Style::rounded());
    println!("{table}");
    let duration = start_time.elapsed();
    println!("This operation took {:?}", duration);
    Ok(())
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let args = CliArgs::parse();
    let sh = Shell::new()?;

    // Get the logs in a format:
    // Hash Date Author
    // e83c5163316f89bfbde7d9ab23ca2e25604af290 2024-09-28T17:45:47+00:00 John Doe
    let raw_output = cmd!(sh, "git log --pretty=format:'%H %aI %an'").read()?;
    if raw_output.is_empty() {
        println!("No commits found.");
        return Ok(());
    }
    // TODO: paginate and batch process commits.
    // If there are hundreds of thousands of commits this may be a bottleneck.
    let commits: Vec<Commit> = raw_output.par_lines().filter_map(parse_commit).collect();

    if args.all && args.only.is_none() {
        print_table(&commits, start_time)
    } else if let Some(only) = args.only {
        let only_commits = commits
            .par_iter()
            .filter(|c| c.rarity.tier == only)
            .cloned()
            .collect::<Vec<Commit>>();
        if only_commits.is_empty() {
            println!("No {} commits found.", only);
            return Ok(());
        }
        return print_table(&only_commits, start_time);
    } else if args.count {
        let count = Count {
            total: commits.len(),
            common: commits
                .par_iter()
                .filter(|c| c.rarity.tier == RarityTier::Common)
                .count(),
            uncommon: commits
                .par_iter()
                .filter(|c| c.rarity.tier == RarityTier::Uncommon)
                .count(),
            rare: commits
                .par_iter()
                .filter(|c| c.rarity.tier == RarityTier::Rare)
                .count(),
        };
        return print_table(&vec![count], start_time);
    } else {
        let not_common_commits = commits
            .par_iter()
            .filter(|c| c.rarity.tier != RarityTier::Common)
            .cloned()
            .collect::<Vec<Commit>>();

        if not_common_commits.is_empty() {
            println!("No uncommon or rare commits found.");
            return Ok(());
        }
        return print_table(&not_common_commits, start_time);
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
