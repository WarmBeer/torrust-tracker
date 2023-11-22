use std::sync::Arc;

use crate::shared::bit_torrent::info_hash::InfoHash;
use crate::tracker::peer;
use crate::tracker::torrent::{Entry, SwarmStats};

pub trait TRepositorySync {
    fn new() -> Self;
    fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &peer::Peer) -> (SwarmStats, bool);
}

pub trait TRepositoryAsync {
    fn new() -> Self;
    fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &peer::Peer) -> impl std::future::Future<Output = (SwarmStats, bool)> + Send;
}

/// Structure that holds all torrents. Using std::sync locks.
pub struct RepositorySync {
    torrents: std::sync::RwLock<std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>>,
}

impl RepositorySync {
    pub fn get_torrents(&self) -> std::sync::RwLockReadGuard<'_, std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>> {
        self.torrents.read().unwrap()
    }

    pub fn get_torrents_mut(&self) -> std::sync::RwLockWriteGuard<'_, std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>> {
        self.torrents.write().unwrap()
    }
}

impl TRepositorySync for RepositorySync {
    fn new() -> Self {
        Self {
            torrents: std::sync::RwLock::new(std::collections::BTreeMap::new())
        }
    }

    fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &peer::Peer) -> (SwarmStats, bool) {
        let maybe_existing_torrent_entry = self.get_torrents().get(info_hash).cloned();

        let torrent_entry: Arc<std::sync::Mutex<Entry>> = if let Some(existing_torrent_entry) = maybe_existing_torrent_entry {
            existing_torrent_entry
        } else {
            let mut torrents_lock = self.get_torrents_mut();
            let entry = torrents_lock
                .entry(*info_hash)
                .or_insert(Arc::new(std::sync::Mutex::new(Entry::new())));
            entry.clone()
        };

        let (stats, stats_updated) = {
            let mut torrent_entry_lock = torrent_entry.lock().unwrap();
            let stats_updated = torrent_entry_lock.update_peer(peer);
            let stats = torrent_entry_lock.get_stats();

            (stats, stats_updated)
        };

        (SwarmStats {
            completed: stats.1,
            seeders: stats.0,
            leechers: stats.2,
        }, stats_updated)
    }
}

/// Structure that holds all torrents. Using std::sync locks.
pub struct RepositorySyncOld {
    torrents: std::sync::RwLock<std::collections::BTreeMap<InfoHash, Entry>>,
}

impl RepositorySyncOld {
    pub fn get_torrents(&self) -> std::sync::RwLockReadGuard<'_, std::collections::BTreeMap<InfoHash, Entry>> {
        self.torrents.read().unwrap()
    }

    pub fn get_torrents_mut(&self) -> std::sync::RwLockWriteGuard<'_, std::collections::BTreeMap<InfoHash, Entry>> {
        self.torrents.write().unwrap()
    }
}

impl TRepositorySync for RepositorySyncOld {
    fn new() -> Self {
        Self {
            torrents: std::sync::RwLock::new(std::collections::BTreeMap::new())
        }
    }

    fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &peer::Peer) -> (SwarmStats, bool) {
        let mut torrents = self.torrents.write().unwrap();

        let torrent_entry = match torrents.entry(*info_hash) {
            std::collections::btree_map::Entry::Vacant(vacant) => vacant.insert(Entry::new()),
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        };

        let stats_updated = torrent_entry.update_peer(peer);
        let stats = torrent_entry.get_stats();

        (SwarmStats {
            completed: stats.1,
            seeders: stats.0,
            leechers: stats.2,
        }, stats_updated)
    }
}

/// Structure that holds all torrents. Using tokio::sync locks.
pub struct RepositoryAsync {
    torrents: tokio::sync::RwLock<std::collections::BTreeMap<InfoHash, Arc<tokio::sync::Mutex<Entry>>>>,
}

impl TRepositoryAsync for RepositoryAsync {
    fn new() -> Self {
        Self {
            torrents: tokio::sync::RwLock::new(std::collections::BTreeMap::new())
        }
    }

    async fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &peer::Peer) -> (SwarmStats, bool) {
        let maybe_existing_torrent_entry = self.get_torrents().await.get(info_hash).cloned();

        let torrent_entry: Arc<tokio::sync::Mutex<Entry>> = if let Some(existing_torrent_entry) = maybe_existing_torrent_entry {
            existing_torrent_entry
        } else {
            let mut torrents_lock = self.get_torrents_mut().await;
            let entry = torrents_lock
                .entry(*info_hash)
                .or_insert(Arc::new(tokio::sync::Mutex::new(Entry::new())));
            entry.clone()
        };

        let (stats, stats_updated) = {
            let mut torrent_entry_lock = torrent_entry.lock().await;
            let stats_updated = torrent_entry_lock.update_peer(peer);
            let stats = torrent_entry_lock.get_stats();

            (stats, stats_updated)
        };

        (SwarmStats {
            completed: stats.1,
            seeders: stats.0,
            leechers: stats.2,
        }, stats_updated)
    }
}

impl RepositoryAsync {
    pub async fn get_torrents(&self) -> tokio::sync::RwLockReadGuard<'_, std::collections::BTreeMap<InfoHash, Arc<tokio::sync::Mutex<Entry>>>> {
        self.torrents.read().await
    }

    pub async fn get_torrents_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, std::collections::BTreeMap<InfoHash, Arc<tokio::sync::Mutex<Entry>>>> {
        self.torrents.write().await
    }
}

/// Structure that holds all torrents. Using a tokio::sync lock for the torrents map and std::sync lock for the inner torrent entry.
pub struct RepositoryAsyncSync {
    torrents: tokio::sync::RwLock<std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>>,
}

impl TRepositoryAsync for RepositoryAsyncSync {
    fn new() -> Self {
        Self {
            torrents: tokio::sync::RwLock::new(std::collections::BTreeMap::new())
        }
    }

    async fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &peer::Peer) -> (SwarmStats, bool) {
        let maybe_existing_torrent_entry = self.get_torrents().await.get(info_hash).cloned();

        let torrent_entry: Arc<std::sync::Mutex<Entry>> = if let Some(existing_torrent_entry) = maybe_existing_torrent_entry {
            existing_torrent_entry
        } else {
            let mut torrents_lock = self.get_torrents_mut().await;
            let entry = torrents_lock
                .entry(*info_hash)
                .or_insert(Arc::new(std::sync::Mutex::new(Entry::new())));
            entry.clone()
        };

        let (stats, stats_updated) = {
            let mut torrent_entry_lock = torrent_entry.lock().unwrap();
            let stats_updated = torrent_entry_lock.update_peer(peer);
            let stats = torrent_entry_lock.get_stats();

            (stats, stats_updated)
        };

        (SwarmStats {
            completed: stats.1,
            seeders: stats.0,
            leechers: stats.2,
        }, stats_updated)
    }
}

impl RepositoryAsyncSync {
    pub async fn get_torrents(&self) -> tokio::sync::RwLockReadGuard<'_, std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>> {
        self.torrents.read().await
    }

    pub async fn get_torrents_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, std::collections::BTreeMap<InfoHash, Arc<std::sync::Mutex<Entry>>>> {
        self.torrents.write().await
    }
}