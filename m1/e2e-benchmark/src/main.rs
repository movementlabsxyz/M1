use anyhow::{Context, Result};
use aptos_sdk::{
    coin_client::CoinClient,
    rest_client::{Client, FaucetClient},
    types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::{
    collections::VecDeque,
    fs::File,
    io::Write,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{self, time as tokio_time};
use url::Url;
use std::str::FromStr;
use tokio::time::{self, sleep};
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

static NODE_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_NODE_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://fullnode.devnet.aptoslabs.com"),
    )
    .unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_FAUCET_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://faucet.devnet.aptoslabs.com"),
    )
    .unwrap()
});

struct Statistics {
    records: Vec<(Instant, bool)>, // (Timestamp, Success)
    max_tps: f64,
    min_tps: f64,
    avg_tps: f64,
}

impl Statistics {
    fn new() -> Self {
        Self {
            records: vec![],
            max_tps: 0.0,
            min_tps: f64::MAX,
            avg_tps: 0.0,
        }
    }

    fn record_transaction(&mut self, success: bool) {
        let now = Instant::now();
        self.records.push((now, success));
    }

    fn analyze_tps(&mut self) {
        let mut tps_values: Vec<f64> = Vec::new();
        // Assuming we calculate TPS over 1-second windows for simplicity
        let start_time = self.records.first().map(|x| x.0).unwrap_or(Instant::now());
        let end_time = self.records.last().map(|x| x.0).unwrap_or(Instant::now());
        let total_duration = end_time.duration_since(start_time).as_secs_f64();
    
        if total_duration > 0.0 {
            let mut current_time = start_time;
            while current_time <= end_time {
                let window_end = current_time + Duration::from_secs(15);
                let count = self
                    .records
                    .iter()
                    .filter(|&&(time, _)| time >= current_time && time < window_end)
                    .count();
                let tps = (count/15) as f64;
                tps_values.push(tps);
    
                current_time += Duration::from_secs(15);
            }
    
            if let Some(max_tps) = tps_values.iter().max_by(|x, y| x.partial_cmp(y).unwrap()) {
                self.max_tps = *max_tps;
            }
            if let Some(min_tps) = tps_values.iter().min_by(|x, y| x.partial_cmp(y).unwrap()) {
                self.min_tps = *min_tps;
            }
    
            // Calculate average TPS
            let sum_tps: f64 = tps_values.iter().sum();
            let avg_tps = if !tps_values.is_empty() { sum_tps / tps_values.len() as f64 } else { 0.0 };
            self.avg_tps = avg_tps; // Make sure to add `avg_tps` to your Statistics struct
        }
    
        // Adjust min_tps if no transactions were recorded to avoid f64::MAX as a value
        if self.min_tps == f64::MAX {
            self.min_tps = 0.0;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup clients and statistics
    let stats = Arc::new(Mutex::new(Statistics::new()));
    // Setup for benchmarking, transaction sending, etc., goes here
    run_simulation(
        stats.clone(),
        Duration::from_secs(60 * 120)
    ).await?;

    // Wait for benchmark to finish
    // Perform analysis
    let mut stats = stats.lock().await;
    stats.analyze_tps();

    println!("Max TPS: {}, Min TPS: {}", stats.max_tps, stats.min_tps);

    // Write statistics to a file
    let mut file = File::create("benchmark_stats.dat")?;
    writeln!(file, "Max TPS: {}", stats.max_tps)?;
    writeln!(file, "Min TPS: {}", stats.min_tps)?;
    for (timestamp, success) in &stats.records {
        writeln!(file, "{}, {}", timestamp.elapsed().as_secs_f64(), if *success { "success" } else { "failure" })?;
    }

    Ok(())
}

// Dummy function to simulate transaction - replace with actual logic
async fn perform_transaction_batch(
    stats: Arc<Mutex<Statistics>>,
) -> Result<()> {

    // :!:>section_1a
    let rest_client = Client::new(NODE_URL.clone());
    let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

    // :!:>section_1b
    let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

    // Create two accounts locally, Alice and Bob.
    // :!:>section_2
    let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
    let mut bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2


    // Create the accounts on chain, but only fund Alice.
    // :!:>section_3
    match faucet_client
    .fund(alice.address(), 100_000_000)
    .await
    .context("Failed to fund Alice's account") {
        
        Ok(_) => {
            let mut stats = stats.lock().await;
            stats.record_transaction(true);
        },
        Err(_) => {
            let mut stats = stats.lock().await;
            stats.record_transaction(false);
        }
    };

    match faucet_client
        .create_account(bob.address())
        .await
        .context("Failed to fund Bob's account") {
            
            Ok(_) => {
                let mut stats = stats.lock().await;
                stats.record_transaction(true);
            },
            Err(_) => {
                let mut stats = stats.lock().await;
                stats.record_transaction(false);
            }
        
    }; 


    // run 16 transfers back and forth
    for _ in 0..16 {
        let txn_hash = coin_client
            .transfer(&mut alice, bob.address(), 1_000, None)
            .await
            .context("Failed to submit transaction to transfer coins")?;
        match rest_client
            .wait_for_transaction(&txn_hash)
            .await
            .context("Failed when waiting for the transfer transaction") {
                
                Ok(_) => {
                    let mut stats = stats.lock().await;
                    stats.record_transaction(true);
                },
                Err(_) => {
                    let mut stats = stats.lock().await;
                    stats.record_transaction(false);
                }
                
        };

        let txn_hash = coin_client
            .transfer(&mut bob, alice.address(), 1_000, None)
            .await
            .context("Failed to submit transaction to transfer coins")?;
        match rest_client
            .wait_for_transaction(&txn_hash)
            .await
            .context("Failed when waiting for the transfer transaction") {
                
                Ok(_) => {
                    let mut stats = stats.lock().await;
                    stats.record_transaction(true);
                },
                Err(_) => {
                    let mut stats = stats.lock().await;
                    stats.record_transaction(false);
                }
                
        };
    } // <:!:section_6

    Ok(())
  
}


async fn run_simulation(stats: Arc<Mutex<Statistics>>, duration: Duration) -> Result<()> {
    let run_flag = Arc::new(AtomicBool::new(true));
    let max_tps = Arc::new(AtomicUsize::new(0));
    let current_tasks = Arc::new(AtomicUsize::new(1024 * 64 * 4)); // 

    // Function to adjust tasks based on performance
    let adjust_tasks = |max_tps: &AtomicUsize, current_tps: usize, current_tasks: &AtomicUsize| {
        if current_tps > max_tps.load(Ordering::Relaxed) {
            max_tps.store(current_tps, Ordering::Relaxed);
            let increment = (current_tps as f64 * 0.1) as usize + 1;
            current_tasks.fetch_add(increment, Ordering::Relaxed);
        } else {
            // If current TPS is significantly lower than max, consider reducing increase rate or stop increasing
            let threshold = max_tps.load(Ordering::Relaxed) * 50 / 100; // 50% of max TPS as threshold
            if current_tps < threshold {
                current_tasks.store(
                    (current_tasks.load(Ordering::Relaxed) as f64 * 0.9) as usize + 4,
                    Ordering::Relaxed,
                );
            }
        }
    };

    let mut handles = Vec::new();
    let stats_clone = stats.clone();
    let run_flag_clone = run_flag.clone();
    let max_tps_clone = max_tps.clone();
    let current_tasks_clone = current_tasks.clone();

    // Background task to adjust the number of parallel tasks
    let adjuster_handle = tokio::spawn(async move {
        while run_flag_clone.load(Ordering::Relaxed) {
            // Wait a bit between adjustments
            sleep(Duration::from_secs(20)).await;

            // Lock stats to read and calculate current TPS
            let mut stats = stats_clone.lock().await;
            stats.analyze_tps();
            let current_tps = stats.avg_tps.round() as usize;

            adjust_tasks(&max_tps_clone, current_tps, &current_tasks_clone);

            // Print current strategy status
            println!("Current TPS: {}, Max TPS: {}, Current Tasks: {}", current_tps, max_tps_clone.load(Ordering::Relaxed), current_tasks_clone.load(Ordering::Relaxed));
        }
    });

    // Main loop to manage tasks based on current_tasks count
    let now = Instant::now();
    while run_flag.load(Ordering::Relaxed) && Instant::now() < now + duration{
        let current_task_count = current_tasks.load(Ordering::Relaxed);
        while handles.len() < current_task_count {
            let stats_clone = stats.clone();
            let run_flag_clone = run_flag.clone();

            let handle = tokio::spawn(async move {
                while run_flag_clone.load(Ordering::Relaxed) {
                    match perform_transaction_batch(stats_clone.clone()).await {
                        Ok(_) => {},
                        Err(_) => {}
                    }
                }
            });

            handles.push(handle);
        }

        // Optionally remove excess handles if current_tasks decreased
        while handles.len() > current_task_count  {
            if let Some(handle) = handles.pop() {
                handle.abort(); // Stop the extra task
            }
        }

        // Sleep a bit before next adjustment check
        sleep(Duration::from_secs(1)).await;
    }

    // Signal all tasks to stop
    run_flag.store(false, Ordering::Relaxed);

    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Ensure the adjuster task is also completed
    let _ = adjuster_handle.await;

    Ok(())
}
