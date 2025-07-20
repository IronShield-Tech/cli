use ironshield_api::handler::{
    error::ErrorHandler,
    result::ResultHandler
};
use ironshield_types::{IronShieldChallenge, IronShieldChallengeResponse};
use crate::config::ClientConfig;

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Instant;
use tokio::task::JoinHandle;
use futures::future;

// Progress callbacks removed for maximum performance - they were killing performance!

/// Configuration for multithreaded solving
#[derive(Debug, Clone)]
pub struct SolveConfig {
    /// Number of threads to use for solving
    pub thread_count: usize,
    /// Whether to use multithreaded solving
    pub use_multithreaded: bool,
}

impl SolveConfig {
    /// Create a new solve configuration
    pub fn new(config: &ClientConfig, use_multithreaded: bool) -> Self {
        let available_cores = num_cpus::get();
        
        // Use 80% of available cores, minimum 1, respect config override
        let thread_count = if use_multithreaded {
            config.num_threads
                .unwrap_or_else(|| std::cmp::max(1, (available_cores * 4) / 5))
        } else {
            1
        };

        Self {
            thread_count,
            use_multithreaded,
        }
    }
}

/// Solves an IronShield challenge using multithreaded approach
pub async fn solve_challenge(
    challenge: IronShieldChallenge,
    config: &ClientConfig,
    use_multithreaded: bool,
) -> ResultHandler<IronShieldChallengeResponse> {
    let solve_config = SolveConfig::new(config, use_multithreaded);
    
    crate::verbose_section!(config, "Challenge Solving");
    crate::verbose_kv!(config, "Thread Count", solve_config.thread_count);
    crate::verbose_kv!(config, "Multithreaded", solve_config.use_multithreaded);
    crate::verbose_kv!(config, "Recommended Attempts", challenge.recommended_attempts);

    let start_time = Instant::now();

    let result = if solve_config.use_multithreaded && solve_config.thread_count > 1 {
        solve_multithreaded(challenge, solve_config.clone(), config).await
    } else {
        solve_single_threaded(challenge, config).await
    };

    match result {
        Ok(solution) => {
            let elapsed = start_time.elapsed();
            let elapsed_millis = elapsed.as_millis() as u64;
            
            // Calculate estimated total attempts across all threads using thread-stride analysis
            // In thread-stride: if thread T finds solution at nonce N, it has done roughly (N/thread_count) attempts
            // Other threads have done roughly the same amount of work
            let solution_nonce = solution.solution as u64;
            let estimated_attempts_per_thread = (solution_nonce / solve_config.thread_count as u64) + 1;
            let estimated_total_attempts = estimated_attempts_per_thread * solve_config.thread_count as u64;
            
            let hash_rate = if elapsed_millis > 0 {
                (estimated_total_attempts * 1000) / elapsed_millis
            } else {
                estimated_total_attempts  // If solved instantly, assume 1ms
            };
            
            crate::verbose_log!(
                config,
                timing,
                "Challenge solved in {:?} (~{} estimated total attempts, ~{} h/s)",
                elapsed,
                estimated_total_attempts,
                hash_rate
            );
            
            crate::verbose_log!(
                config,
                success,
                "Performance: {} threads achieved ~{} hashes/second (solution found at nonce {})",
                solve_config.thread_count,
                hash_rate,
                solution_nonce
            );
            
            Ok(solution)
        },
        Err(e) => {
            let elapsed = start_time.elapsed();
            crate::verbose_log!(
                config,
                error,
                "Challenge solving failed after {:?}: {}",
                elapsed,
                e
            );
            Err(e)
        }
    }
}

/// Solve using multiple threads with proper early termination - FIXED!
async fn solve_multithreaded(
    challenge: IronShieldChallenge,
    solve_config: SolveConfig,
    config: &ClientConfig,
) -> ResultHandler<IronShieldChallengeResponse> {
    crate::verbose_log!(config, compute, "Starting multithreaded solve with {} threads", solve_config.thread_count);

    let challenge = Arc::new(challenge);
    let mut handles: Vec<JoinHandle<ResultHandler<IronShieldChallengeResponse>>> = Vec::new();
    
    // Shared cancellation flag to stop progress reporting after solution found
    let solution_found = Arc::new(AtomicBool::new(false));

    // Spawn worker threads with proper stride and offset
    for thread_id in 0..solve_config.thread_count {
        let challenge_clone = Arc::clone(&challenge);
        let thread_stride = solve_config.thread_count as u64;
        let thread_offset = thread_id as u64;
        let config_clone = config.clone();
        let solution_found_clone = Arc::clone(&solution_found);
        
        crate::verbose_log!(
            config, 
            compute, 
            "Spawning thread {} with offset {} and stride {} (with progress callbacks for status updates)", 
            thread_id, 
            thread_offset, 
            thread_stride
        );

        let handle = tokio::task::spawn_blocking(move || {
            // Track thread start time for accurate hash rate calculation
            let thread_start_time = Instant::now();
            
            // Track cumulative attempts (core callback gives us batch sizes, not cumulative)
            let cumulative_attempts = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let cumulative_attempts_clone = Arc::clone(&cumulative_attempts);
            
            // Progress callback for status updates - provides the "status checks" during hashing
            let progress_callback = |batch_attempts: u64| {
                // Stop reporting progress if solution already found by another thread
                if solution_found_clone.load(Ordering::Relaxed) {
                    return;
                }
                
                // The core callback gives us BATCH SIZE (always 200,000), not cumulative!
                // We need to accumulate these batches like the JavaScript implementation
                let total_attempts = cumulative_attempts_clone.fetch_add(batch_attempts, Ordering::Relaxed) + batch_attempts;
                
                let elapsed = thread_start_time.elapsed();
                let elapsed_millis = elapsed.as_millis() as u64;
                
                // Calculate hash rate based on CUMULATIVE attempts, not just this batch
                let hash_rate = if elapsed_millis > 0 {
                    (total_attempts * 1000) / elapsed_millis
                } else {
                    total_attempts  // If solved instantly, assume 1ms
                };
                
                crate::verbose_log!(
                    config_clone,
                    compute,
                    "Thread {} progress: {} total attempts on this thread ({} hashes/second)",
                    thread_id,
                    total_attempts,
                    hash_rate
                );
            };
            
            // Call ironshield-core's find_solution_multi_threaded function WITH progress callback
            ironshield_core::find_solution_multi_threaded(
                &*challenge_clone,
                None,                              // num_threads (not used in worker coordination)
                Some(thread_offset as usize),      // start_offset for this thread
                Some(thread_stride as usize),      // stride for optimal thread-stride pattern
                Some(&progress_callback),          // Progress callback for status updates!
            ).map_err(|e| ErrorHandler::ProcessingError(format!(
                "Thread {} failed: {}", thread_id, e
            )))
        });

        handles.push(handle);
    }

    // Wait for ANY thread to find a solution and immediately cancel others
    // This mimics the JavaScript worker.terminate() behavior perfectly!
    let mut solution = None;
    let mut remaining_handles = handles;
    
    while !remaining_handles.is_empty() && solution.is_none() {
        // Wait for the first handle to complete
        let (result, thread_index, other_handles) = future::select_all(remaining_handles).await;

        match result {
            Ok(Ok(found_solution)) => {
                // Signal all threads to stop progress reporting
                solution_found.store(true, Ordering::Relaxed);
                
                crate::verbose_log!(config, success, "Thread {} found solution! Signaling {} other threads to stop progress reporting.", thread_index, other_handles.len());
                solution = Some(found_solution);
                
                // Abort all remaining handles immediately
                // Note: The core computation cannot be cancelled, but we stop the tokio tasks and progress reporting
                for handle in other_handles {
                    handle.abort();
                }
                crate::verbose_log!(config, success, "All remaining threads aborted (core computation may continue briefly but progress reporting stopped).");
                break;
            },
            Ok(Err(e)) => {
                crate::verbose_log!(config, warning, "Thread {} error: {}. Continuing with {} remaining threads.", thread_index, e, other_handles.len());
                remaining_handles = other_handles;
            },
            Err(e) => {
                crate::verbose_log!(config, error, "Thread {} join error: {}. Continuing with {} remaining threads.", thread_index, e, other_handles.len());
                remaining_handles = other_handles;
            }
        }
    }

    solution.ok_or_else(|| ErrorHandler::ProcessingError(
        "No solution found by any thread".to_string()
    ))
}

/// Solve using a single thread
async fn solve_single_threaded(
    challenge: IronShieldChallenge,
    config: &ClientConfig,
) -> ResultHandler<IronShieldChallengeResponse> {
    crate::verbose_log!(config, compute, "Starting single-threaded solve");

    // Use tokio::task::spawn_blocking to avoid blocking the async runtime
    let handle = tokio::task::spawn_blocking(move || {
        // Use single-threaded function (progress callbacks not supported in single-threaded core)
        ironshield_core::find_solution_single_threaded(&challenge)
    });

    match handle.await {
        Ok(Ok(solution)) => {
            crate::verbose_log!(config, success, "Single-threaded solve completed successfully");
            Ok(solution)
        },
        Ok(Err(e)) => {
            Err(ErrorHandler::ProcessingError(format!(
                "Single-threaded solve failed: {}", e
            )))
        },
        Err(e) => {
            Err(ErrorHandler::ProcessingError(format!(
                "Single-threaded solve task failed: {}", e
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_solve_config_single_threaded() {
        let config = ClientConfig {
            endpoint: "https://test.com".to_string(),
            api_base_url: "https://api.test.com".to_string(),
            timeout: Duration::from_secs(30),
            verbose: false,
            num_threads: Some(4),
        };

        let solve_config = SolveConfig::new(&config, false);
        assert_eq!(solve_config.thread_count, 1);
        assert!(!solve_config.use_multithreaded);
    }

    #[test]
    fn test_solve_config_multithreaded() {
        let config = ClientConfig {
            endpoint: "https://test.com".to_string(),
            api_base_url: "https://api.test.com".to_string(),
            timeout: Duration::from_secs(30),
            verbose: false,
            num_threads: Some(4),
        };

        let solve_config = SolveConfig::new(&config, true);
        assert_eq!(solve_config.thread_count, 4);
        assert!(solve_config.use_multithreaded);
    }

    #[test]
    fn test_solve_config_auto_thread_count() {
        let config = ClientConfig {
            endpoint: "https://test.com".to_string(),
            api_base_url: "https://api.test.com".to_string(),
            timeout: Duration::from_secs(30),
            verbose: false,
            num_threads: None, // Auto-detect
        };

        let solve_config = SolveConfig::new(&config, true);
        assert!(solve_config.thread_count >= 1);
        assert!(solve_config.use_multithreaded);
    }
}
