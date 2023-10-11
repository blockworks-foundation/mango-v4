use super::*;

use itertools::Itertools;
use regex::Regex;

#[tokio::test]
async fn test_benchmark() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let result = send_tx_get_metadata(solana, BenchmarkInstruction {})
        .await
        .unwrap();
    let meta = result.metadata.unwrap();

    let log_lines = meta.log_messages;
    let bench_regions = log_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            if line.starts_with(&"Program log: BENCH") {
                Some(index)
            } else {
                None
            }
        })
        .chain([log_lines.len()])
        .collect_vec();

    let name_regex = Regex::new(r#"BENCH: (.+)"#).unwrap();
    let cu_regex = Regex::new(r#"(\d+) units remaining"#).unwrap();
    for (start, end) in bench_regions.iter().tuple_windows() {
        let lines = &log_lines[*start..*end];
        let name = name_regex.captures(&lines[0]).unwrap()[1].to_string();

        let cu = lines
            .iter()
            .filter_map(|line| {
                cu_regex
                    .captures(line)
                    .map(|c| c[1].parse::<u64>().unwrap())
            })
            .take(2)
            .collect_vec();
        let cu_print_cost = 101;
        let cost = cu[0] - cu[1] - cu_print_cost;
        println!("{name:<25}{cost:>7}");
    }

    Ok(())
}
