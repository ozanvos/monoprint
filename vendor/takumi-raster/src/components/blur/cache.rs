//! Bounded reuse of compact alpha masks. Keys contain actual pre-blur bytes;
//! hashes only accelerate lookup, so collisions cannot return another mask.
use std::{
  collections::VecDeque,
  sync::{Arc, Mutex, OnceLock},
};
use xxhash_rust::xxh3::xxh3_64;

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
  width: u32,
  height: u32,
  radius: u32,
  hash: u64,
}

struct Entry {
  key: Key,
  input: Arc<[u8]>,
  output: Arc<[u8]>,
}

#[derive(Default)]
struct Cache {
  entries: VecDeque<Entry>,
  bytes: usize,
}

impl Cache {
  fn get(&mut self, key: Key, input: &[u8]) -> Option<Arc<[u8]>> {
    let index = self
      .entries
      .iter()
      .position(|e| e.key == key && &*e.input == input)?;
    let entry = self.entries.remove(index)?;
    let output = entry.output.clone();
    self.entries.push_front(entry);
    Some(output)
  }

  fn insert(&mut self, entry: Entry) {
    let bytes = entry.input.len() + entry.output.len();
    if bytes > MAX_BYTES || self.get(entry.key, &entry.input).is_some() {
      return;
    }
    while self.bytes + bytes > MAX_BYTES || self.entries.len() >= MAX_ENTRIES {
      let Some(old) = self.entries.pop_back() else {
        break;
      };
      self.bytes -= old.input.len() + old.output.len();
    }
    self.bytes += bytes;
    self.entries.push_front(entry);
  }
}

pub(super) fn blur(data: &mut [u8], params: super::BlurPassParams) {
  static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
  // Retain multiple useful masks rather than let one large mask occupy the
  // entire budget. Rendering remains valid when caching is skipped/poisoned.
  if data.len() > MAX_BYTES / 8 {
    super::blur_alpha_passes(data, params);
    return;
  }
  let key = Key {
    width: params.width,
    height: params.height,
    radius: params.radius,
    hash: xxh3_64(data),
  };
  let cache = CACHE.get_or_init(|| Mutex::new(Cache::default()));
  if let Some(output) = cache.lock().ok().and_then(|mut c| c.get(key, data)) {
    data.copy_from_slice(&output);
    return;
  }
  let input = Arc::from(&*data);
  // Never hold the cache lock while computing a miss.
  super::blur_alpha_passes(data, params);
  if let Ok(mut cache) = cache.lock() {
    cache.insert(Entry {
      key,
      input,
      output: Arc::from(&*data),
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn collisions_geometry_and_budget_do_not_reuse_another_mask() {
    let mut cache = Cache::default();
    let key = Key {
      width: 1,
      height: 1,
      radius: 1,
      hash: 0,
    };
    let entry = |input, output| Entry {
      key,
      input: Arc::from([input]),
      output: Arc::from([output]),
    };
    cache.insert(entry(1, 2));
    cache.insert(entry(3, 4));
    assert_eq!(&*cache.get(key, &[1]).unwrap(), &[2]);
    assert!(cache.get(Key { radius: 2, ..key }, &[1]).is_none());
    for value in 5..100 {
      cache.insert(entry(value, value));
    }
    assert_eq!(cache.entries.len(), MAX_ENTRIES);
    assert!(cache.get(key, &[1]).is_none());
    let large = vec![7; MAX_BYTES / 4];
    for radius in 1..=4 {
      cache.insert(Entry {
        key: Key { radius, ..key },
        input: Arc::from(&*large),
        output: Arc::from(&*large),
      });
      assert!(cache.bytes <= MAX_BYTES);
    }
    assert_eq!(cache.entries.len(), 2);
  }
}
