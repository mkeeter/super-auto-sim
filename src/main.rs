use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use log::{debug, info, LevelFilter};
use serde::{Deserialize, Serialize};

mod battle;
mod food;
mod friend;
mod modifier;
mod params;
mod rng;
mod shop;
mod species;
mod team;

use battle::{Battle, Winner};
use shop::Shop;
use team::Team;

////////////////////////////////////////////////////////////////////////////////

const TEAMS_FILE: &str = "teams.ron";
const SCORES_FILE: &str = "scores.ron";

fn generate_teams() {
    let mut seen_teams = HashSet::new();
    let mut seen_shops = HashSet::new();
    let mut rng = rng::Rng::new();
    while rng.next() {
        let mut shop = Shop::new(&mut rng);
        let new = loop {
            if !seen_shops.insert(shop) {
                break false;
            }
            if shop.step(&mut rng) {
                break true;
            }
        };
        if new {
            let mut team = shop.team;
            team.compact();
            if seen_teams.insert(team) {
                debug!("New team:\n{}", team);
            }
        }
    }
    info!("Saving {} teams to '{}'", seen_teams.len(), TEAMS_FILE);
    let seen: Vec<Team> = seen_teams.into_iter().collect();
    std::fs::write(
        TEAMS_FILE,
        ron::to_string(&seen).expect("Failed to serialize teams"),
    )
    .expect("Failed to save teams");
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct Record {
    wins: f32,
    ties: f32,
}
fn score_teams(teams: Vec<Team>) {
    let mut results: HashMap<usize, HashMap<usize, Record>> = HashMap::new();
    for (i, a) in teams.iter().enumerate() {
        for (j, b) in teams.iter().enumerate() {
            let battle = Battle(*a, *b);
            let num_battles = 1; // TODO

            let mut team_a = 0.0;
            let mut team_b = 0.0;
            let mut ties = 0.0;
            for _ in 0..num_battles {
                match battle.run() {
                    Winner::TeamA => team_a += 1.0,
                    Winner::TeamB => team_b += 1.0,
                    Winner::Tied => ties += 1.0,
                }
            }
            let num_battles = num_battles as f32;
            results.entry(i).or_default().insert(
                j,
                Record {
                    wins: team_a / num_battles,
                    ties: ties / num_battles,
                },
            );
            results.entry(j).or_default().insert(
                i,
                Record {
                    wins: team_b / num_battles,
                    ties: ties / num_battles,
                },
            );
        }
        let mut num_wins = 0.0;
        let mut num_ties = 0.0;
        let mut count = 0.0;
        for r in results[&i].values() {
            num_wins += r.wins;
            num_ties += r.ties;
            count += 1.0;
        }
        debug!(
            "Team {} wins {:.1}% and draws {:.1}%:\n{}",
            i,
            num_wins / count * 100.0,
            num_ties / count * 100.0,
            teams[i]
        );
    }
    std::fs::write(
        SCORES_FILE,
        ron::to_string(&results).expect("Failed to serialize scores"),
    )
    .expect("Failed to save scores");
    // TODO
}

fn analyze_scores(
    teams: Vec<Team>,
    results: HashMap<usize, HashMap<usize, Record>>,
) {
    let mut most_wins = 0.0;
    let mut best_team = 0;

    for (k, v) in &results {
        let mut num_wins = 0.0;
        let mut count = 0.0;
        for (j, r) in v {
            num_wins += r.wins;
            count += 1.0;
        }
        if num_wins / count > most_wins {
            best_team = *k;
            most_wins = num_wins / count;
        }
    }
    println!(
        "The team with the most wins ({:.2}%) [{}]:\n{}",
        most_wins * 100.0,
        best_team,
        teams[best_team]
    );
}

fn main() {
    use env_logger::Builder;

    let mut log = Builder::new();

    let args = std::env::args();
    match args.len() {
        1 => {
            log.filter_level(LevelFilter::Debug);
            log.parse_env("RUST_LOG");
            log.init();

            generate_teams();
            /*
            if let Ok(d) = std::fs::read_to_string(TEAMS_FILE) {
                let teams: Vec<Team> = ron::from_str(&d).unwrap();
                if let Ok(d) = std::fs::read_to_string(SCORES_FILE) {
                    let scores = ron::from_str(&d).unwrap();
                    analyze_scores(teams, scores);
                } else {
                    score_teams(teams);
                }
            } else {
                generate_teams();
            }
            */
        }
        2 => {
            // By default, when asked to generate a team, print the verbose
            // team generation log.
            log.filter_level(LevelFilter::Trace);
            log.parse_env("RUST_LOG");
            log.init();

            //let team = random_team(seed);
            //debug!("Got team [{}]:\n{}", seed, team);
        }
        i => {
            panic!("Invalid argument count {}", i);
        }
    }
}
