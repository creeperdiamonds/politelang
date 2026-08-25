//! `polite bench` — the budgets from spec 10.4, measured on this machine.
//!
//! Spec 10.4 on enforcement: continuous integration cannot run on the reference laptop, so
//! numbers from a cloud runner would be meaningless. The baseline lives in the repository,
//! measured here, and a run fails if anything has slipped by more than a tenth.

use crate::pipeline;

/// Timings are the *best* of several runs, not the average.
///
/// On a two-core laptop something else is always waking up, and an average measures whatever else
/// the machine happened to be doing. The fastest run is the one where the machine got out of the
/// way, and it is the only one that says anything about the code.
fn best_of(rounds: u32, mut run: impl FnMut() -> f64) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..rounds {
        let taken = run();
        if taken < best {
            best = taken;
        }
    }
    best
}

use polite_vocab::Vocabulary;
use std::process::ExitCode;
use std::time::Instant;

const BASELINE: &str = "bench-baseline.txt";

struct Measure {
    name: &'static str,
    value: f64,
    unit: &'static str,
    /// Whether a bigger number is better.
    bigger_is_better: bool,
    budget: Option<f64>,
    /// How much movement is beneath notice.
    ///
    /// A measurement that is meant to sit near zero cannot be judged by percentages: going from
    /// 0.1% to 1% is a tenfold worsening of nothing at all. This is the amount that has to move
    /// before anybody should care.
    slack: f64,
}

pub fn run(args: &[String], vocab: &Vocabulary) -> ExitCode {
    let save = args.iter().any(|a| a == "--save");
    let mut results: Vec<Measure> = Vec::new();

    println!("Measuring on this machine. Nothing else should be busy.\n");

    // ---- Checking throughput -------------------------------------------------
    let program = synthetic_program(1000);
    let mut tree_bytes = 0usize;
    let mut checked_out = true;
    let each = best_of(7, || {
        let start = Instant::now();
        let built = pipeline::build("bench.polite", &program, vocab, true);
        let taken = start.elapsed().as_secs_f64();
        tree_bytes = built.tree_bytes;
        if built.had_problems {
            checked_out = false;
        }
        taken
    });
    if !checked_out {
        eprintln!("The benchmark program did not check out, so the numbers would be a lie.");
        return ExitCode::FAILURE;
    }
    results.push(Measure {
        name: "check a 1,000 line program",
        value: each * 1000.0,
        unit: "ms",
        bigger_is_better: false,
        budget: Some(10.0),
        slack: 0.5,
    });
    results.push(Measure {
        name: "checking throughput",
        value: 1000.0 / each,
        unit: "lines/sec",
        bigger_is_better: true,
        budget: Some(150_000.0),
        slack: 10_000.0,
    });
    results.push(Measure {
        name: "tree memory per line",
        value: tree_bytes as f64 / 1000.0,
        unit: "bytes",
        bigger_is_better: false,
        budget: Some(2048.0),
        slack: 16.0,
    });

    // ---- Hello, end to end, without the process start ------------------------
    let hello = "please show \"hello\"\n";
    let each = best_of(200, || {
        let start = Instant::now();
        let built = pipeline::build("hello.polite", hello, vocab, true);
        let p = built.program.expect("hello should build");
        let mut world = polite_run::Scripted::default();
        let _ = polite_run::run(&p, &mut world);
        start.elapsed().as_secs_f64()
    });
    results.push(Measure {
        name: "compile and run hello",
        value: each * 1000.0,
        unit: "ms",
        bigger_is_better: false,
        budget: Some(3.0),
        slack: 0.05,
    });

    // ---- Ten times the vocabulary -------------------------------------------
    // Spec 10.4 calls this the one budget that tests a claim rather than a speed: section 4 says
    // a large vocabulary is cheap, and either that is true or the central bet is wrong.
    let big = grow_vocabulary(4000);
    let mut ratios: Vec<f64> = Vec::with_capacity(21);
    for _ in 0..21 {
        // Measured as a pair, one straight after the other, so whatever the machine is doing at
        // the time lands on both sides of the comparison.
        let small_time = time_parse(vocab, &program);
        let big_time = time_parse(&big, &program);
        ratios.push(big_time / small_time.max(f64::MIN_POSITIVE));
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let slowdown = (ratios[ratios.len() / 2] - 1.0) * 100.0;
    results.push(Measure {
        name: "parse slowdown at 40x the vocabulary",
        value: slowdown.max(0.0),
        unit: "%",
        bigger_is_better: false,
        budget: Some(5.0),
        slack: 3.0,
    });

    // ---- Numeric loop, against CPython ---------------------------------------
    let loop_src = "please remember total is 0\nplease repeat for every n from 1 to 300000:\n    add n to total\nthanks\nplease show total\n";
    let built = pipeline::build("loop.polite", loop_src, vocab, true);
    match built.program {
        Some(p) => {
            let taken = best_of(5, || {
                let start = Instant::now();
                let mut world = polite_run::Scripted::default();
                let _ = polite_run::run(&p, &mut world);
                start.elapsed().as_secs_f64()
            });
            results.push(Measure {
                name: "300,000 turn numeric loop",
                value: taken * 1000.0,
                unit: "ms",
                bigger_is_better: false,
                budget: None,
                slack: 0.0,
            });
        }
        None => println!("{}", built.messages),
    }

    // ---- The same loop, in CPython -------------------------------------------
    // Spec 10.4 budgets the interpreter against CPython. Comparing against a number written down
    // months ago would be worthless, so the comparison is run here, now, on this machine.
    match cpython_loop_ms() {
        Some(python_ms) => {
            let ours = results
                .iter()
                .find(|m| m.name == "300,000 turn numeric loop")
                .map(|m| m.value)
                .unwrap_or(python_ms);
            results.push(Measure {
                name: "same loop in CPython",
                value: python_ms,
                unit: "ms",
                bigger_is_better: false,
                budget: None,
                slack: 0.0,
            });
            results.push(Measure {
                name: "times faster than CPython",
                value: python_ms / ours.max(0.0001),
                unit: "x",
                bigger_is_better: true,
                budget: Some(2.0),
                slack: 0.5,
            });
        }
        None => println!(
            "(CPython is not on the path, so the comparison in spec 10.4 was not run.)
"
        ),
    }

    // ---- Report --------------------------------------------------------------
    let previous = load_baseline();
    let mut slipped = Vec::new();

    println!("{:<42}{:>14}{:>12}{:>10}", "", "measured", "budget", "before");
    for m in &results {
        let budget = match m.budget {
            Some(b) => format!("{}", pretty(b)),
            None => "-".to_string(),
        };
        let before = previous
            .iter()
            .find(|(n, _)| n == m.name)
            .map(|(_, v)| pretty(*v))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<42}{:>14}{:>12}{:>10}   {}",
            m.name,
            format!("{} {}", pretty(m.value), m.unit),
            budget,
            before,
            verdict(m)
        );

        if let Some((_, was)) = previous.iter().find(|(n, _)| n == m.name) {
            let moved = (m.value - was).abs();
            let worse = moved > m.slack
                && if m.bigger_is_better {
                    m.value < was * 0.9
                } else {
                    m.value > was * 1.1
                };
            if worse {
                slipped.push(m.name);
            }
        }
    }

    if save {
        let text: String = results
            .iter()
            .map(|m| format!("{}\t{}\n", m.name, m.value))
            .collect();
        match std::fs::write(BASELINE, text) {
            Ok(()) => println!("\nBaseline written to {BASELINE}."),
            Err(e) => eprintln!("\nI could not write {BASELINE}: {e}"),
        }
        return ExitCode::SUCCESS;
    }

    let missed: Vec<&str> = results
        .iter()
        .filter(|m| match m.budget {
            Some(b) => {
                if m.bigger_is_better {
                    m.value < b
                } else {
                    m.value > b
                }
            }
            None => false,
        })
        .map(|m| m.name)
        .collect();

    println!();
    if !slipped.is_empty() {
        println!("Slower than last time by more than a tenth: {slipped:?}");
    }
    if !missed.is_empty() {
        println!("Outside the budget from the spec: {missed:?}");
        return ExitCode::FAILURE;
    }
    if slipped.is_empty() {
        println!("Everything is inside its budget.");
    }
    ExitCode::SUCCESS
}

fn verdict(m: &Measure) -> &'static str {
    match m.budget {
        None => "",
        Some(b) => {
            let ok = if m.bigger_is_better {
                m.value >= b
            } else {
                m.value <= b
            };
            if ok {
                "ok"
            } else {
                "over"
            }
        }
    }
}

fn pretty(v: f64) -> String {
    if v >= 10_000.0 {
        format!("{:.0}", v)
    } else if v >= 100.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}

fn load_baseline() -> Vec<(String, f64)> {
    let text = match std::fs::read_to_string(BASELINE) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .filter_map(|l| {
            let mut parts = l.splitn(2, '\t');
            let name = parts.next()?.to_string();
            let value: f64 = parts.next()?.trim().parse().ok()?;
            Some((name, value))
        })
        .collect()
}

fn time_parse(vocab: &Vocabulary, source: &str) -> f64 {
    best_of(5, || {
        let start = Instant::now();
        let _ = polite_syntax::parse(source, vocab);
        start.elapsed().as_secs_f64()
    })
}

/// The same loop, timed in CPython, if CPython is here to ask.
fn cpython_loop_ms() -> Option<f64> {
    let script = "
import time
best = 1e9
for _ in range(5):
    t = time.perf_counter()
    total = 0
    for n in range(1, 300001):
        total += n
    best = min(best, (time.perf_counter() - t) * 1000)
print(best)
";
    for exe in ["python", "python3", "py"] {
        if let Ok(out) = std::process::Command::new(exe).arg("-c").arg(script).output() {
            if out.status.success() {
                if let Ok(text) = String::from_utf8(out.stdout) {
                    if let Ok(ms) = text.trim().parse::<f64>() {
                        return Some(ms);
                    }
                }
            }
        }
    }
    None
}

fn synthetic_program(lines: usize) -> String {
    let mut s = String::with_capacity(lines * 30);
    s.push_str("please remember total is 0\n");
    s.push_str("please remember names is an empty list\n");
    for i in 0..lines {
        match i % 5 {
            0 => s.push_str(&format!("please add {} to total\n", i % 17 + 1)),
            1 => s.push_str(&format!("please add \"name{i}\" to names\n")),
            2 => s.push_str("please check if total is over 10:\n    show total\nthanks\n"),
            3 => s.push_str(&format!(
                "please repeat {} times:\n    add 1 to total\nthanks\n",
                i % 3 + 1
            )),
            _ => s.push_str("please show \"the total is {total}\"\n"),
        }
    }
    s
}

/// Build a vocabulary with thousands of extra phrases in it, all reaching the same handful of
/// forms, and see whether parsing notices.
fn grow_vocabulary(extra: usize) -> Vocabulary {
    let mut text = String::from(include_str!("../../../vocabulary/core.vocab"));
    for i in 0..extra {
        text.push_str(&format!("stmt full show :: zz{i}word {{value}}\n"));
    }
    Vocabulary::load(&text).expect("the grown vocabulary should still load")
}
