//! Small, non-cryptographic hasher used for the internal type maps.
//!
//! Keys are type identities (static Rust paths and type names) rather than
//! attacker-controlled input, so the DoS resistance of SipHash is not needed.
//! Hashing type paths shows up in resolution profiles because every named type
//! and every instance attribute goes through a hash map.

use std::hash::{BuildHasherDefault, Hasher};

/// Multiplier from rustc's FxHash.
const K: u64 = 0x517c_c1b7_2722_0a95;

/// A fast, deterministic hasher for small string keys.
#[derive(Default, Clone, Copy)]
pub(crate) struct FastHasher(u64);

impl FastHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(K);
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for chunk in chunks {
            self.add(u64::from_ne_bytes(*chunk));
        }

        if !rest.is_empty() {
            let mut word = [0u8; 8];
            word[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_ne_bytes(word));
        }
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add(value as u64);
    }

    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.add(value as u64);
    }

    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.add(value as u64);
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }
}

pub(crate) type FastBuildHasher = BuildHasherDefault<FastHasher>;
pub(crate) type FastHashMap<K, V> = std::collections::HashMap<K, V, FastBuildHasher>;
