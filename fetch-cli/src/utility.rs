use tokio::runtime::RuntimeMetrics;

pub fn print_metrics(metrics: &RuntimeMetrics) {
    println!("Runtime Metrics:");
    
    // Task and Worker Metrics
    println!("  - Num workers: {}", metrics.num_workers());
    println!("  - Num alive tasks: {}", metrics.num_alive_tasks());
    println!("  - Spawned tasks count: {}", metrics.spawned_tasks_count());
    println!("  - Budget forced yield count: {}", metrics.budget_forced_yield_count());
    
    // Queue Metrics
    println!("  - Global queue depth: {}", metrics.global_queue_depth());
    println!("  - Blocking queue depth: {}", metrics.blocking_queue_depth());
    
    // Worker-specific metrics for each worker
    for worker in 0..metrics.num_workers() {
        println!("  Worker {}:", worker);
        println!("    - Local queue depth: {}", metrics.worker_local_queue_depth(worker));
        println!("    - Local schedule count: {}", metrics.worker_local_schedule_count(worker));
        println!("    - Overflow count: {}", metrics.worker_overflow_count(worker));
        println!("    - Total busy duration: {:?}", metrics.worker_total_busy_duration(worker));
        println!("    - Park count: {}", metrics.worker_park_count(worker));
        println!("    - Steal count: {}", metrics.worker_steal_count(worker));
        println!("    - Poll count: {}", metrics.worker_poll_count(worker));
        println!("    - Noop count: {}", metrics.worker_noop_count(worker));
        println!("    - Mean poll time: {:?}", metrics.worker_mean_poll_time(worker));
    }
    
    // I/O and System Metrics
    println!("  - Remote schedule count: {}", metrics.remote_schedule_count());
    println!("  - IO driver fd registered count: {}", metrics.io_driver_fd_registered_count());
    println!("  - IO driver fd deregistered count: {}", metrics.io_driver_fd_deregistered_count());
    println!("  - IO driver ready count: {}", metrics.io_driver_ready_count());
    
    // Thread Pool Metrics
    println!("  - Num blocking threads: {}", metrics.num_blocking_threads());
    println!("  - Num idle blocking threads: {}", metrics.num_idle_blocking_threads());
}