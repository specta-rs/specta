//! A counting allocator, so a test can measure what the derive allocates.
//!
//! Only compiled for `cargo test`. It is installed as the crate's
//! `#[global_allocator]` (see `lib.rs`) and stays disabled unless a measuring
//! helper turns it on, so the rest of the test suite pays only two thread-local
//! reads per allocation.
//!
//! The counters are thread-local: the test harness runs tests in parallel and
//! an expansion's allocations all happen on the thread that calls it, so this
//! needs no locking and no `--test-threads=1`.
//!
//! Caveat, and the reason the numbers are only ever compared *relatively*: a
//! unit test links `proc-macro2`'s fallback implementation, not the
//! compiler-provided bridge. Real derives hand tokens across the bridge, which
//! copies them; the fallback keeps them in an `Rc`. Absolute allocation counts
//! therefore differ from a real build (they understate the token traffic), but
//! two builds of this crate measured the same way are directly comparable.

// `GlobalAlloc` is an unsafe trait by definition; this module is test-only and
// forwards every operation to the system allocator.
#![allow(unsafe_code)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
};

/// The allocator installed for the test binary.
pub struct CountingAllocator;

thread_local! {
    /// Whether the current frame is being measured. Nested `measure` calls keep
    /// the outermost flag and just add to its counters.
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };

    /// The guards currently open on this thread, outermost first.
    ///
    /// A guard snapshots the counters when it opens; on drop it reports
    /// `current - snapshot` (which includes everything its children counted)
    /// and hands that inclusive figure to its parent, so a parent can report its
    /// own work by subtracting.
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };

    /// Every closed guard's numbers, drained by the harness after a measurement.
    static LOG: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}

/// How many guards are expected open at once; the storage is reserved before
/// counting starts so the guards themselves are never attributed to a phase.
const FRAME_CAPACITY: usize = 32;
/// How many closed guards one measurement is expected to log.
const LOG_CAPACITY: usize = 16 * 1024;

/// Record one allocation of `bytes` bytes, if a measurement is running.
///
/// `try_with` rather than `with`: the allocator can be called while a thread's
/// TLS is being destroyed, when accessing these cells would panic (and the
/// panic would allocate, recursively).
fn record(bytes: usize) {
    let _ = ENABLED.try_with(|enabled| {
        if enabled.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
            let _ = BYTES.try_with(|bytes_cell| bytes_cell.set(bytes_cell.get() + bytes));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    // A reallocation is one allocation of the *new* size. `realloc` moves the
    // contents, so the bytes it touches are the new layout's, not the delta.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// The allocation count and byte total of one measured closure.
#[derive(Copy, Clone, Debug, Default)]
pub struct Measured {
    /// Number of allocation calls (`alloc`, `alloc_zeroed` and `realloc`).
    pub allocations: usize,
    /// Bytes requested, summed over those calls.
    pub bytes: usize,
}

impl Measured {
    /// This measurement minus `other`, saturating at zero.
    ///
    /// Used to separate the parse a `derive_tokens` call does internally from
    /// its expansion: both run the identical `syn` parse of the identical
    /// tokens, so the difference is exactly the expansion's own work.
    pub fn saturating_sub(self, other: Self) -> Self {
        Self {
            allocations: self.allocations.saturating_sub(other.allocations),
            bytes: self.bytes.saturating_sub(other.bytes),
        }
    }

    pub fn checked_add(self, other: Self) -> Self {
        Self {
            allocations: self.allocations + other.allocations,
            bytes: self.bytes + other.bytes,
        }
    }
}

struct Frame {
    /// Everything the children of this frame counted, so it can report its own
    /// share separately.
    child_allocations: usize,
    child_bytes: usize,
}

/// One closed guard: what it counted inclusively and what is left when its
/// children are subtracted.
#[derive(Copy, Clone)]
pub struct Entry {
    pub label: &'static str,
    pub inclusive: Measured,
    pub own: Measured,
}

/// Turn counting on and reserve the guard storage, unless a measurement is
/// already running.
///
/// Returns whether this call is the outermost one, i.e. whether it is the one
/// that has to turn counting back off.
fn begin() -> bool {
    if !FRAMES.with(|frames| frames.borrow().is_empty()) {
        return false;
    }

    // Reserve before counting: growing these vectors is the harness's own cost
    // and must not show up in whichever phase happens to open first.
    FRAMES.with(|frames| frames.borrow_mut().reserve(FRAME_CAPACITY));
    LOG.with(|log| log.borrow_mut().reserve(LOG_CAPACITY));
    ALLOCATIONS.with(|count| count.set(0));
    BYTES.with(|bytes| bytes.set(0));
    ENABLED.with(|enabled| enabled.set(true));
    true
}

/// Run `f`, returning its result and what it allocated.
pub fn measure<T>(f: impl FnOnce() -> T) -> (T, Measured) {
    // A measurement nested inside another one (a closure that itself calls
    // `measure`) attributes everything to the outer one, which is what its
    // caller wants.
    if !begin() {
        return (f(), Measured::default());
    }

    // A frame of its own, so the phases `f` attributes are not mistaken for the
    // outermost guard and turn counting off when they end.
    let guard = open(None);
    let value = f();
    let measured = Measured {
        allocations: ALLOCATIONS.with(Cell::get),
        bytes: BYTES.with(Cell::get),
    };
    drop(guard);
    (value, measured)
}

/// Attributes everything that happens until it is dropped to `label`.
///
/// Guards nest: an inner guard's allocations are also part of every guard
/// around it, so [`Entry::own`] subtracts the children out. The harness drains
/// the log after a measurement; nothing is returned here because [`Drop`]
/// cannot hand a value back.
pub struct Guard {
    /// `None` for the guard [`measure`] opens, which exists only to keep the
    /// stack balanced.
    label: Option<&'static str>,
    start_allocations: usize,
    start_bytes: usize,
}

/// Open a [`Guard`] for `label`.
///
/// Must be called inside a [`measure`]; the harness only ever does.
pub fn guard(label: &'static str) -> Guard {
    open(Some(label))
}

fn open(label: Option<&'static str>) -> Guard {
    // `measure` has already turned counting on and reserved the storage, so
    // this push does not allocate.
    let (start_allocations, start_bytes) = (ALLOCATIONS.with(Cell::get), BYTES.with(Cell::get));
    FRAMES.with(|frames| {
        frames.borrow_mut().push(Frame {
            child_allocations: 0,
            child_bytes: 0,
        })
    });

    Guard {
        label,
        start_allocations,
        start_bytes,
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let inclusive = Measured {
            allocations: ALLOCATIONS.with(Cell::get) - self.start_allocations,
            bytes: BYTES.with(Cell::get) - self.start_bytes,
        };

        let frame = FRAMES
            .with(|frames| frames.borrow_mut().pop())
            .expect("guard dropped without being opened");

        let own = Measured {
            allocations: inclusive.allocations - frame.child_allocations,
            bytes: inclusive.bytes - frame.child_bytes,
        };

        let outermost = FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            match frames.last_mut() {
                Some(parent) => {
                    parent.child_allocations += inclusive.allocations;
                    parent.child_bytes += inclusive.bytes;
                    false
                }
                None => true,
            }
        });

        if outermost {
            ENABLED.with(|enabled| enabled.set(false));
        }

        // The guard `measure` opens has no label of its own, but its share is
        // worth reporting: it is the work done between the named phases.
        LOG.with(|log| {
            log.borrow_mut().push(Entry {
                label: self.label.unwrap_or("unattributed"),
                inclusive,
                own,
            })
        });
    }
}

/// Take everything logged since the last call, so the harness can aggregate it.
pub fn take_log() -> Vec<Entry> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}
