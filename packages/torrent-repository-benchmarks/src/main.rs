use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use aquatic_udp_protocol::{AnnounceEvent, NumberOfBytes};
use clap::Parser;
use futures::stream::FuturesUnordered;
use torrust_tracker::shared::bit_torrent::info_hash::InfoHash;
use torrust_tracker::shared::clock::DurationSinceUnixEpoch;
use torrust_tracker::tracker::peer::{Id, Peer};
use torrust_tracker::tracker::torrent::repository::{TRepositorySync, RepositorySync, RepositoryAsync, TRepositoryAsync, RepositorySyncOld, RepositoryAsyncSync};

/// Simple program to benchmark torrust-tracker
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Amount of benchmark worker threads
    #[arg(short, long)]
    threads: usize,
    /// Amount of time in ns a thread will sleep to simulate a client response after handling a task
    #[arg(short, long)]
    sleep: Option<u64>,
    /// Compare with old implementations of the torrent repository
    #[arg(short, long)]
    compare: Option<bool>,
}

fn main() {
    let args = Args::parse();

    // Add 1 to worker_threads since we need a thread that awaits the benchmark
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads + 1)
        .enable_time()
        .build()
        .unwrap();

    println!("{}: Avg/AdjAvg: {:?}", "async_sync_add_one_torrent", rt.block_on(async_add_one_torrent::<RepositoryAsyncSync>(1_000_000)));
    println!("{}: Avg/AdjAvg: {:?}", "async_sync_update_one_torrent_in_parallel", rt.block_on(async_update_one_torrent_in_parallel::<RepositoryAsyncSync>(&rt, 10)));
    println!("{}: Avg/AdjAvg: {:?}", "async_sync_add_multiple_torrents_in_parallel", rt.block_on(async_add_multiple_torrents_in_parallel::<RepositoryAsyncSync>(&rt, 10)));
    println!("{}: Avg/AdjAvg: {:?}", "async_sync_update_multiple_torrents_in_parallel", rt.block_on(async_update_multiple_torrents_in_parallel::<RepositoryAsyncSync>(&rt, 10)));

    if let Some(true) = args.compare {
        println!("");

        println!("{}: Avg/AdjAvg: {:?}", "async_add_one_torrent", rt.block_on(async_add_one_torrent::<RepositoryAsync>(1_000_000)));
        println!("{}: Avg/AdjAvg: {:?}", "async_update_one_torrent_in_parallel", rt.block_on(async_update_one_torrent_in_parallel::<RepositoryAsync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "async_add_multiple_torrents_in_parallel", rt.block_on(async_add_multiple_torrents_in_parallel::<RepositoryAsync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "async_update_multiple_torrents_in_parallel", rt.block_on(async_update_multiple_torrents_in_parallel::<RepositoryAsync>(&rt, 10)));

        println!("");

        println!("{}: Avg/AdjAvg: {:?}", "add_one_torrent", add_one_torrent::<RepositorySync>(1_000_000));
        println!("{}: Avg/AdjAvg: {:?}", "update_one_torrent_in_parallel", rt.block_on(update_one_torrent_in_parallel::<RepositorySync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "add_multiple_torrents_in_parallel", rt.block_on(add_multiple_torrents_in_parallel::<RepositorySync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "update_multiple_torrents_in_parallel", rt.block_on(update_multiple_torrents_in_parallel::<RepositorySync>(&rt, 10)));

        println!("");

        println!("{}: Avg/AdjAvg: {:?}", "single_lock_add_one_torrent", add_one_torrent::<RepositorySyncOld>(1_000_000));
        println!("{}: Avg/AdjAvg: {:?}", "single_lock_update_one_torrent_in_parallel", rt.block_on(update_one_torrent_in_parallel::<RepositorySyncOld>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "single_lock_add_multiple_torrents_in_parallel", rt.block_on(add_multiple_torrents_in_parallel::<RepositorySyncOld>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "single_lock_update_multiple_torrents_in_parallel", rt.block_on(update_multiple_torrents_in_parallel::<RepositorySyncOld>(&rt, 10)));
    }
}

const DEFAULT_PEER: Peer = Peer {
    peer_id: Id([0; 20]),
    peer_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
    updated: DurationSinceUnixEpoch::from_secs(0),
    uploaded: NumberOfBytes(0),
    downloaded: NumberOfBytes(0),
    left: NumberOfBytes(0),
    event: AnnounceEvent::Started,
};

fn generate_unique_info_hashes(size: usize) -> Vec<InfoHash> {
    let mut result = HashSet::new();

    let mut bytes = [0u8; 20];

    for i in 0..size {
        bytes[0] = (i & 0xFF) as u8;
        bytes[1] = ((i >> 8) & 0xFF) as u8;
        bytes[2] = ((i >> 16) & 0xFF) as u8;
        bytes[3] = ((i >> 24) & 0xFF) as u8;

        let info_hash = InfoHash(bytes);
        result.insert(info_hash);
    }

    assert_eq!(result.len(), size);

    result.into_iter().collect()
}

fn within_acceptable_range(test: &Duration, norm: &Duration) -> bool {
    let test_secs = test.as_secs_f64();
    let norm_secs = norm.as_secs_f64();

    // Calculate the upper and lower bounds for the 10% tolerance
    let tolerance = norm_secs * 0.1;

    // Calculate the upper and lower limits
    let upper_limit = norm_secs + tolerance;
    let lower_limit = norm_secs - tolerance;

    test_secs < upper_limit && test_secs > lower_limit
}

fn get_average_and_adjusted_average_from_results(mut results: Vec<Duration>) -> (Duration, Duration) {
    let average = results.iter().sum::<Duration>() / results.len() as u32;

    results.retain(|result| within_acceptable_range(result, &average));

    let mut adjusted_average = Duration::from_nanos(0);

    if results.len() > 1 {
        adjusted_average = results.iter().sum::<Duration>() / results.len() as u32;
    }

    (average, adjusted_average)
}

// Simply add one torrent
fn add_one_torrent<T: TRepositorySync + Send + Sync + 'static>(samples: usize) -> (Duration, Duration) {
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());

        let info_hash = InfoHash([0; 20]);

        let start_time = std::time::Instant::now();

        torrent_repository.update_torrent_with_peer_and_get_stats(&info_hash, &DEFAULT_PEER);

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

// Add one torrent ten thousand times in parallel (depending on the set worker threads)
async fn update_one_torrent_in_parallel<T: TRepositorySync + Send + Sync + 'static>(runtime: &tokio::runtime::Runtime, samples: usize) -> (Duration, Duration) {
    let args = Args::parse();
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());
        let info_hash: &'static InfoHash = &InfoHash([0; 20]);
        let handles = FuturesUnordered::new();

        // Add the torrent/peer to the torrent repository
        torrent_repository.update_torrent_with_peer_and_get_stats(info_hash, &DEFAULT_PEER);

        let start_time = std::time::Instant::now();

        for _ in 0..10_000 {
            let torrent_repository_clone = torrent_repository.clone();

            let handle = runtime.spawn(async move {
                torrent_repository_clone.update_torrent_with_peer_and_get_stats(info_hash, &DEFAULT_PEER);

                if let Some(sleep_time) = args.sleep {
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_nanos() < sleep_time as u128 {}
                }
            });

            handles.push(handle);
        }

        // Await all tasks
        futures::future::join_all(handles).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

// Add ten thousand torrents in parallel (depending on the set worker threads)
async fn add_multiple_torrents_in_parallel<T: TRepositorySync + Send + Sync + 'static>(runtime: &tokio::runtime::Runtime, samples: usize) -> (Duration, Duration) {
    let args = Args::parse();
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());
        let info_hashes = generate_unique_info_hashes(10_000);
        let handles = FuturesUnordered::new();

        let start_time = std::time::Instant::now();

        for info_hash in info_hashes {
            let torrent_repository_clone = torrent_repository.clone();

            let handle = runtime.spawn(async move {
                torrent_repository_clone.update_torrent_with_peer_and_get_stats(&info_hash, &DEFAULT_PEER);

                if let Some(sleep_time) = args.sleep {
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_nanos() < sleep_time as u128 {}
                }
            });

            handles.push(handle);
        }

        // Await all tasks
        futures::future::join_all(handles).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

// Update ten thousand torrents in parallel (depending on the set worker threads)
async fn update_multiple_torrents_in_parallel<T: TRepositorySync + Send + Sync + 'static>(runtime: &tokio::runtime::Runtime, samples: usize) -> (Duration, Duration) {
    let args = Args::parse();
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());
        let info_hashes = generate_unique_info_hashes(10_000);
        let handles = FuturesUnordered::new();

        // Add the torrents/peers to the torrent repository
        for info_hash in info_hashes.iter() {
            torrent_repository.update_torrent_with_peer_and_get_stats(info_hash, &DEFAULT_PEER);
        }

        let start_time = std::time::Instant::now();

        for info_hash in info_hashes {
            let torrent_repository_clone = torrent_repository.clone();

            let handle = runtime.spawn(async move {
                torrent_repository_clone.update_torrent_with_peer_and_get_stats(&info_hash, &DEFAULT_PEER);

                if let Some(sleep_time) = args.sleep {
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_nanos() < sleep_time as u128 {}
                }
            });

            handles.push(handle);
        }

        // Await all tasks
        futures::future::join_all(handles).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

async fn async_add_one_torrent<T: TRepositoryAsync + Send + Sync + 'static>(samples: usize) -> (Duration, Duration) {
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());

        let info_hash = InfoHash([0; 20]);

        let start_time = std::time::Instant::now();

        torrent_repository.update_torrent_with_peer_and_get_stats(&info_hash, &DEFAULT_PEER).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

// Add one torrent ten thousand times in parallel (depending on the set worker threads)
async fn async_update_one_torrent_in_parallel<T: TRepositoryAsync + Send + Sync + 'static>(runtime: &tokio::runtime::Runtime, samples: usize) -> (Duration, Duration) {
    let args = Args::parse();
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());
        let info_hash: &'static InfoHash = &InfoHash([0; 20]);
        let handles = FuturesUnordered::new();

        // Add the torrent/peer to the torrent repository
        torrent_repository.update_torrent_with_peer_and_get_stats(info_hash, &DEFAULT_PEER).await;

        let start_time = std::time::Instant::now();

        for _ in 0..10_000 {
            let torrent_repository_clone = torrent_repository.clone();

            let handle = runtime.spawn(async move {
                torrent_repository_clone.update_torrent_with_peer_and_get_stats(info_hash, &DEFAULT_PEER).await;

                if let Some(sleep_time) = args.sleep {
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_nanos() < sleep_time as u128 {}
                }
            });

            handles.push(handle);
        }

        // Await all tasks
        futures::future::join_all(handles).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

// Add ten thousand torrents in parallel (depending on the set worker threads)
async fn async_add_multiple_torrents_in_parallel<T: TRepositoryAsync + Send + Sync + 'static>(runtime: &tokio::runtime::Runtime, samples: usize) -> (Duration, Duration) {
    let args = Args::parse();
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(T::new());
        let info_hashes = generate_unique_info_hashes(10_000);
        let handles = FuturesUnordered::new();

        let start_time = std::time::Instant::now();

        for info_hash in info_hashes {
            let torrent_repository_clone = torrent_repository.clone();

            let handle = runtime.spawn(async move {
                torrent_repository_clone.update_torrent_with_peer_and_get_stats(&info_hash, &DEFAULT_PEER).await;

                if let Some(sleep_time) = args.sleep {
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_nanos() < sleep_time as u128 {}
                }
            });

            handles.push(handle);
        }

        // Await all tasks
        futures::future::join_all(handles).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}

// Async update ten thousand torrents in parallel (depending on the set worker threads)
async fn async_update_multiple_torrents_in_parallel<T: TRepositoryAsync + Send + Sync + 'static>(runtime: &tokio::runtime::Runtime, samples: usize) -> (Duration, Duration) {
    let args = Args::parse();
    let mut results: Vec<Duration> = Vec::with_capacity(samples);

    for _ in 0..samples {
        let torrent_repository = Arc::new(RepositoryAsync::new());
        let info_hashes = generate_unique_info_hashes(10_000);
        let handles = FuturesUnordered::new();

        // Add the torrents/peers to the torrent repository
        for info_hash in info_hashes.iter() {
            torrent_repository.update_torrent_with_peer_and_get_stats(info_hash, &DEFAULT_PEER).await;
        }

        let start_time = std::time::Instant::now();

        for info_hash in info_hashes {
            let torrent_repository_clone = torrent_repository.clone();

            let handle = runtime.spawn(async move {
                torrent_repository_clone.update_torrent_with_peer_and_get_stats(&info_hash, &DEFAULT_PEER).await;

                if let Some(sleep_time) = args.sleep {
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_nanos() < sleep_time as u128 {}
                }
            });

            handles.push(handle);
        }

        // Await all tasks
        futures::future::join_all(handles).await;

        let result = start_time.elapsed();

        results.push(result);
    }

    get_average_and_adjusted_average_from_results(results)
}