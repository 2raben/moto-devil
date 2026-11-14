//! The browser store: one local storage item per entry, such as
//! `moto-devil/progress`. See ADR 0010.
//!
//! All pages of an origin share its local storage; on GitHub Pages that is every
//! project page of the account. The prefix avoids accidental collisions, and the
//! texts are decoded as untrusted input like files.

use std::io;

use super::{Entry, Store};

const PREFIX: &str = "moto-devil/";

pub struct BrowserStore;

impl BrowserStore {
    /// The page's local storage, or `None` if the browser has none or blocks it.
    pub fn open() -> Option<Self> {
        storage().ok().map(|_| Self)
    }
}

/// Fetched for every call: the browser object cannot be kept in a resource,
/// which must be shareable between threads.
fn storage() -> io::Result<web_sys::Storage> {
    let window = web_sys::window().ok_or_else(|| io::Error::other("no window"))?;
    window
        .local_storage()
        .map_err(js_error)?
        .ok_or_else(|| io::Error::other("no local storage"))
}

fn js_error(error: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("{error:?}"))
}

fn key(entry: Entry) -> String {
    format!("{PREFIX}{}", entry.name())
}

impl Store for BrowserStore {
    fn read(&self, entry: Entry) -> io::Result<Option<String>> {
        let text = storage()?.get_item(&key(entry)).map_err(js_error)?;
        if let Some(text) = &text {
            entry.check_size(text.len())?;
        }
        Ok(text)
    }

    fn write(&mut self, entry: Entry, text: &str) -> io::Result<()> {
        // Fails without changing the old item, for example when the quota is full.
        storage()?.set_item(&key(entry), text).map_err(js_error)
    }

    fn back_up(&mut self, entry: Entry) -> io::Result<()> {
        let storage = storage()?;
        let key = key(entry);
        if let Some(text) = storage.get_item(&key).map_err(js_error)? {
            storage
                .set_item(&format!("{key}.broken"), &text)
                .map_err(js_error)?;
        }
        storage.remove_item(&key).map_err(js_error)
    }

    fn describe(&self, entry: Entry) -> String {
        format!("local storage item `{}`", key(entry))
    }
}
