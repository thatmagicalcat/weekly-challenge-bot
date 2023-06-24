use std::io::Write;
use std::process::{Command, Stdio};
use std::time;

pub fn test(program_cmd: &str, test_cases: &str) -> Option<f64> {
    let contents = test_cases.lines().collect::<Vec<_>>();

    let number_of_lines_per_testcases = contents.first().unwrap().parse::<usize>().unwrap();
    let number_of_testcases = (contents.len() - 1) / number_of_lines_per_testcases;

    let all_testcases = contents[1..].iter().copied().step_by(2).collect::<Vec<_>>();
    let answers = contents[1..]
        .iter()
        .copied()
        .skip(1)
        .step_by(2)
        .collect::<Vec<_>>();

    let timer = time::Instant::now();
    let mut dest = Command::new(program_cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    dest.stdin
        .take()
        .unwrap()
        .write_all(format!("{number_of_testcases}\n{}", all_testcases.join("\n")).as_bytes())
        .unwrap();

    let output = dest.wait_with_output().unwrap();
    let time_taken = timer.elapsed().as_micros() as f64 / 1000000.0;

    let output = String::from_utf8_lossy(&output.stdout);
    let output = output.lines().collect::<Vec<_>>();

    // check if the output is correct or not
    for idx in 0..answers.len() {
        if answers.get(idx) != output.get(idx) {
            return None;
        }
    }

    Some(time_taken)
}
