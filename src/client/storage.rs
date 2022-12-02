use super::options::Options;
use super::session::Session;
use log::warn;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;
use std::fs::File;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Storage<T> {
    path: PathBuf,
    value: T,
}

impl Storage<Session> {
    pub fn for_session(options: Options) -> Self {
        let path = session_profile_path(&options.profile);
        let (value, save) = match Storage::load(&path) {
            Ok(session) if options.matches(&session) => (session, false),
            _ => (options.create_session(), true),
        };

        let resource = Storage { path, value };
        if save {
            resource.save();
        }

        resource
    }
}

impl<T: DeserializeOwned + Serialize> Storage<T> {
    fn load(path: &Path) -> io::Result<T> {
        let file = File::open(path)?;
        let value = serde_json::from_reader(file)?;

        Ok(value)
    }

    fn save(&self) {
        if let Err(err) = self.try_save() {
            warn!("error saving to '{}': {:?}", self.path.display(), err);
        }
    }

    fn try_save(&self) -> io::Result<()> {
        let file = File::create(&self.path)?;
        serde_json::to_writer_pretty(file, &self.value)?;

        Ok(())
    }

    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.value);
        self.save();
    }
}

impl<T> Deref for Storage<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

fn default_wallectconnect_cache_dir() -> PathBuf {
    let mut cache = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| {
                let mut home = PathBuf::from(home);
                home.push(".cache");
                home
            })
        })
        .unwrap_or_else(|| PathBuf::from("/etc/cache"));
    cache.push("walletconnect-rs");
    cache
}

fn session_profile_path(profile: &Path) -> PathBuf {
    let mut path = default_wallectconnect_cache_dir();
    path.push("profiles");
    path.push(profile);
    path.set_extension("json");
    path
}
