use hashbrown::{HashMap, HashSet};
use log::{debug, info, trace, LevelFilter};
use serde::{Deserialize, Serialize};

mod battle;
mod dice;
mod food;
mod friend;
mod modifier;
mod params;
mod shop;
mod species;
mod team;
mod util;

use battle::{Battle, Winner};
use dice::DeterministicDice;
use params::TEAM_SIZE;
use shop::Shop;
use team::Team;
use util::{read_compressed, write_compressed};

////////////////////////////////////////////////////////////////////////////////

const TEAMS_FILE: &str = "teams.binz";
const SCORES_FILE: &str = "scores.binz";

fn generate_teams() -> Vec<Team> {
    let mut active_shops = HashSet::new();
    let mut seen_shops = HashMap::new();
    let mut dice = DeterministicDice::new();
    while dice.next() {
        active_shops.insert(Shop::new(&mut dice));
    }
    info!("Got {} initial shops", active_shops.len());

    let mut seen_teams = HashSet::new();
    while !active_shops.is_empty() {
        let num_shops = active_shops.len();
        info!(
            "Got {} active shops, {} teams, {} seen shops",
            num_shops,
            seen_teams.len(),
            seen_shops.len()
        );
        let mut next = HashSet::new();
        for (i, shop) in active_shops.into_iter().enumerate() {
            trace!("Running on shop {} / {}", i + 1, num_shops);
            // If we've already seen this shop, and had more gold when we last
            // encountered it, then this branch isn't going to generate anything
            // worthwhile.
            let mut shop_without_gold = shop;
            shop_without_gold.gold = 0;
            if let Some(prev_gold) = seen_shops.get(&shop_without_gold) {
                if *prev_gold >= shop.gold {
                    trace!("Duplicate shop; exiting");
                    continue;
                }
            }
            seen_shops.insert(shop_without_gold, shop.gold);

            let mut dice = DeterministicDice::new();
            while dice.next() {
                let mut shop = shop;
                let done = shop.step(&mut dice);
                let team = shop.team;
                // Do an early check here to make sure we haven't seen this
                // shop before, _before_ building every possible permutation
                if next.contains(&shop) {
                    continue;
                }

                // Store all possible compact permutations of this team, to
                // minimize the amount of exploration required.
                for team in team.compact_permutations() {
                    shop.team = team;
                    if seen_teams.insert(team.without_exp()) {
                        debug!(
                            "New {}team ({}):\n{}",
                            if team.is_dumb() { "(dumb) " } else { "" },
                            seen_teams.len(),
                            team
                        );
                    }
                    if !done {
                        next.insert(shop);
                    }
                }
            }
        }
        active_shops = next;
    }
    let mut seen: Vec<Team> =
        seen_teams.into_iter().filter(|t| !t.is_dumb()).collect();
    info!("Got {} non-dumb teams", seen.len());
    seen.sort();
    seen
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
struct Record {
    wins: f32,
    loses: f32,
    ties: f32,
}
fn score_teams(teams: &[Team]) -> Vec<Vec<Record>> {
    let mut results = vec![vec![Record::default(); teams.len()]; teams.len()];
    let mut max_battles = 0;
    for (i, a) in teams.iter().enumerate() {
        for (j, b) in teams.iter().enumerate() {
            let mut team_a = 0;
            let mut team_b = 0;
            let mut ties = 0;
            let mut num_battles = 0;
            let mut dice = DeterministicDice::new();
            while dice.next() {
                let battle = Battle(*a, *b);
                match battle.run(&mut dice) {
                    Winner::TeamA => team_a += 1,
                    Winner::TeamB => team_b += 1,
                    Winner::Tied => ties += 1,
                }
                num_battles += 1;
            }
            if num_battles > max_battles {
                info!(
                    "New best: {} {} {}\n{}\n{}",
                    team_a, ties, num_battles, a, b
                );
                max_battles = num_battles;
            }
            results[i][j] = Record {
                wins: team_a as f32 / num_battles as f32,
                loses: team_b as f32 / num_battles as f32,
                ties: ties as f32 / num_battles as f32,
            };
        }
        let mut num_wins = 0.0;
        let mut num_ties = 0.0;
        let mut count = 0.0;
        for r in &results[i] {
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
    results
}

fn analyze_scores(teams: Vec<Team>, results: Vec<Vec<Record>>) {
    let mut most_wins = 0.0;
    let mut best_team = 0;

    let mut win_percent = vec![];
    for (k, v) in results.iter().enumerate() {
        let mut num_wins = 0.0;
        let mut count = 0.0;
        for (_j, r) in v.iter().enumerate() {
            num_wins += r.wins;
            count += 1.0;
        }
        if num_wins / count > most_wins {
            best_team = k;
            most_wins = num_wins / count;
        }
        win_percent.push((num_wins / count, teams[k]));
    }
    win_percent.sort_by_key(|k| (-k.0 * 1000000.0) as i32);

    for i in win_percent.iter().take(10) {
        println!("Win percent: {}\n{}\n", i.0, i.1);
    }
    println!(
        "The team with the most wins ({:.2}%) [{}]:\n{}",
        most_wins * 100.0,
        best_team,
        teams[best_team]
    );

    for (k, t) in win_percent.iter().rev() {
        let mut count = 0;
        for i in 0..TEAM_SIZE {
            if t[i].is_some() {
                count += 1;
                if t[i].unwrap().modifier.is_some() {
                    count += 1;
                }
            }
        }
        if count == 3 {
            println!(
                "The worst team with three friends ({:.2}%):\n{}",
                k * 100.0,
                t,
            );
            break;
        }
    }
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

            let teams = match read_compressed(TEAMS_FILE) {
                Some(teams) => {
                    info!("Loading teams from cache");
                    teams
                }
                None => {
                    info!("Generating teams");
                    let teams = generate_teams();
                    write_compressed(&teams, TEAMS_FILE);
                    teams
                }
            };
            let scores = match read_compressed(SCORES_FILE) {
                Some(scores) => {
                    info!("Loading scores from cache");
                    scores
                }
                None => {
                    info!("Scoring teams");
                    let scores = score_teams(&teams);
                    write_compressed(&scores, SCORES_FILE);
                    scores
                }
            };
            info!("Analyzing scores");
            analyze_scores(teams, scores);
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
