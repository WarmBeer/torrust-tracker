use clap::Parser;
use torrust_torrent_repository_benchmarks::args::Args;
use torrust_torrent_repository_benchmarks::benches::asyn::{async_add_multiple_torrents_in_parallel, async_add_one_torrent, async_update_multiple_torrents_in_parallel, async_update_one_torrent_in_parallel};
use torrust_torrent_repository_benchmarks::benches::sync::{add_multiple_torrents_in_parallel, add_one_torrent, update_multiple_torrents_in_parallel, update_one_torrent_in_parallel};
use torrust_tracker::tracker::torrent::repository::{RepositorySync, RepositoryAsync, RepositorySyncSingle, RepositoryAsyncSync, RepositoryAsyncSingle};

fn main() {
    let args = Args::parse();

    // Add 1 to worker_threads since we need a thread that awaits the benchmark
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads + 1)
        .enable_time()
        .build()
        .unwrap();

    println!("tokio::sync::RwLock<std::collections::BTreeMap<InfoHash, Entry>>");
    println!("{}: Avg/AdjAvg: {:?}", "add_one_torrent", rt.block_on(async_add_one_torrent::<RepositoryAsyncSingle>(1_000_000)));
    println!("{}: Avg/AdjAvg: {:?}", "update_one_torrent_in_parallel", rt.block_on(async_update_one_torrent_in_parallel::<RepositoryAsyncSingle>(&rt, 10)));
    println!("{}: Avg/AdjAvg: {:?}", "add_multiple_torrents_in_parallel", rt.block_on(async_add_multiple_torrents_in_parallel::<RepositoryAsyncSingle>(&rt, 10)));
    println!("{}: Avg/AdjAvg: {:?}", "update_multiple_torrents_in_parallel", rt.block_on(async_update_multiple_torrents_in_parallel::<RepositoryAsyncSingle>(&rt, 10)));

    if let Some(true) = args.compare {
        println!("");

        println!("std::sync::RwLock<std::collections::BTreeMap<InfoHash, Entry>>");
        println!("{}: Avg/AdjAvg: {:?}", "add_one_torrent", add_one_torrent::<RepositorySyncSingle>(1_000_000));
        println!("{}: Avg/AdjAvg: {:?}", "update_one_torrent_in_parallel", rt.block_on(update_one_torrent_in_parallel::<RepositorySyncSingle>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "add_multiple_torrents_in_parallel", rt.block_on(add_multiple_torrents_in_parallel::<RepositorySyncSingle>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "update_multiple_torrents_in_parallel", rt.block_on(update_multiple_torrents_in_parallel::<RepositorySyncSingle>(&rt, 10)));

        println!("");

        println!("std::sync::RwLock<std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>>");
        println!("{}: Avg/AdjAvg: {:?}", "add_one_torrent", add_one_torrent::<RepositorySync>(1_000_000));
        println!("{}: Avg/AdjAvg: {:?}", "update_one_torrent_in_parallel", rt.block_on(update_one_torrent_in_parallel::<RepositorySync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "add_multiple_torrents_in_parallel", rt.block_on(add_multiple_torrents_in_parallel::<RepositorySync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "update_multiple_torrents_in_parallel", rt.block_on(update_multiple_torrents_in_parallel::<RepositorySync>(&rt, 10)));

        println!("");

        println!("tokio::sync::RwLock<std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>>");
        println!("{}: Avg/AdjAvg: {:?}", "add_one_torrent", rt.block_on(async_add_one_torrent::<RepositoryAsyncSync>(1_000_000)));
        println!("{}: Avg/AdjAvg: {:?}", "update_one_torrent_in_parallel", rt.block_on(async_update_one_torrent_in_parallel::<RepositoryAsyncSync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "add_multiple_torrents_in_parallel", rt.block_on(async_add_multiple_torrents_in_parallel::<RepositoryAsyncSync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "update_multiple_torrents_in_parallel", rt.block_on(async_update_multiple_torrents_in_parallel::<RepositoryAsyncSync>(&rt, 10)));

        println!("");

        println!("tokio::sync::RwLock<std::collections::BTreeMap<InfoHash, Arc<tokio::sync::Mutex<Entry>>>>");
        println!("{}: Avg/AdjAvg: {:?}", "add_one_torrent", rt.block_on(async_add_one_torrent::<RepositoryAsync>(1_000_000)));
        println!("{}: Avg/AdjAvg: {:?}", "update_one_torrent_in_parallel", rt.block_on(async_update_one_torrent_in_parallel::<RepositoryAsync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "add_multiple_torrents_in_parallel", rt.block_on(async_add_multiple_torrents_in_parallel::<RepositoryAsync>(&rt, 10)));
        println!("{}: Avg/AdjAvg: {:?}", "update_multiple_torrents_in_parallel", rt.block_on(async_update_multiple_torrents_in_parallel::<RepositoryAsync>(&rt, 10)));
    }
}