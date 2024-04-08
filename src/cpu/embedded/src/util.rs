use crate::binding::extern_c::*;
use crate::binding::*;
use core::hint::black_box;
use core::sync::atomic::{compiler_fence, Ordering};
use cty::*;
use micro_blossom_nostd::heapless::Vec;

/// note that it might not be safe to sleep for a long time depending on the C implementation
pub fn sleep(duration: f32) {
    let mut start = unsafe { extern_c::get_native_time() };
    let mut global_diff = 0.;
    // note: this complex implementation is needed because some timer implementation is not capable of
    // recording long time difference, e.g., in Versal board A72 they have 32 bit timer clocked at 150MHz: only capable
    // of recording 28.6s difference. We need to actively accumulating the global timer.
    loop {
        compiler_fence(Ordering::SeqCst);
        let end = unsafe { extern_c::get_native_time() };
        let local_diff = unsafe { extern_c::diff_native_time(start, end) };
        let diff = global_diff + local_diff;
        // avoid overflow by moving the start every 0.5s
        if local_diff > 0.5 {
            start = end;
            global_diff += local_diff;
        }
        if diff >= duration {
            return;
        }
    }
}

pub struct Benchmarker<F>
where
    F: FnMut(),
{
    pub routine: F,
    pub batch_size: usize,
    pub inner_loops: usize,
    /// each train step takes at least these amount of time
    pub train_time: f64,
    /// each batch will approximately take this amount of time
    pub batch_time: f64,
}

impl<F> Benchmarker<F>
where
    F: FnMut(),
{
    pub fn new(routine: F) -> Self {
        Self {
            routine,
            batch_size: 1,
            inner_loops: 1,
            train_time: if cfg![feature = "tiny_benchmark_time"] { 1e-5 } else { 0.1 },
            batch_time: if cfg![feature = "tiny_benchmark_time"] { 1e-4 } else { 1.0 },
        }
    }

    /// run a batch of benchmarker function, so that each batch takes at least 1s
    /// (but no more than 10s to avoid timer overflow)
    pub fn autotune(&mut self) {
        print!("[benchmarker] autotune ... ");
        // find proper batch size
        let mut batch_size = 1;
        loop {
            compiler_fence(Ordering::SeqCst);
            let start = unsafe { extern_c::get_native_time() };
            compiler_fence(Ordering::SeqCst);
            for _ in 0..batch_size {
                black_box((self.routine)());
            }
            compiler_fence(Ordering::SeqCst);
            let end = unsafe { extern_c::get_native_time() };
            let diff = unsafe { extern_c::diff_native_time(start, end) } as f64;
            if diff > 10. {
                println!("the routine takes too long, potentially cause timer overflow, abort");
                panic!();
            }
            if diff >= self.train_time {
                batch_size = core::cmp::max(1, (self.batch_time / diff * (batch_size as f64)) as usize);
                break;
            }
            batch_size *= 2;
        }
        self.batch_size = batch_size;
        println!("batch size = {batch_size}");
    }

    pub fn run(&mut self, round: usize) {
        for batch_idx in 0..round {
            let start = unsafe { extern_c::get_native_time() };
            for _ in 0..self.batch_size {
                black_box((self.routine)());
            }
            let end = unsafe { extern_c::get_native_time() };
            let diff = unsafe { extern_c::diff_native_time(start, end) } as f64;
            let time_per_op = diff / (self.batch_size as f64) / (self.inner_loops as f64);
            println!(
                "[{}/{round}] per_op: {:.2} ns, freq: {:.5} MHz",
                batch_idx + 1,
                time_per_op * 1.0e9,
                1.0e-6 / time_per_op,
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConflictsStore<const MAX_CONFLICT_CHANNELS: usize = 8> {
    pub channels: uint8_t,
    pub cursor: uint8_t,
    pub head: ReadoutHead,
    conflicts: Vec<ReadoutConflict, MAX_CONFLICT_CHANNELS>,
}

impl<const MAX_CC: usize> ConflictsStore<MAX_CC> {
    pub const fn new() -> Self {
        Self {
            channels: 0,
            cursor: 0,
            head: ReadoutHead::new(),
            conflicts: Vec::new(),
        }
    }

    pub fn reconfigure(&mut self, channels: uint8_t) {
        assert!(channels as usize <= MAX_CC);
        self.channels = channels;
        self.cursor = channels;
        self.conflicts.resize_default(channels.into()).unwrap();
    }

    #[inline]
    pub unsafe fn get_conflicts(&mut self, context_id: uint16_t) {
        get_conflicts(&mut self.head, self.conflicts.as_mut_ptr(), self.channels, context_id);
        if self.head.growable == 0 {
            self.reloaded();
        }
    }

    pub fn reloaded(&mut self) {
        self.cursor = 0;
    }

    pub fn pop(&mut self) -> Option<&ReadoutConflict> {
        while self.cursor < self.channels {
            let cursor = self.cursor as usize;
            self.cursor += 1; // always increment
            if self.conflicts[cursor].is_valid() {
                return Some(&self.conflicts[cursor]);
            }
        }
        None
    }

    /// conflict may be uninitialized or outdated, use with care
    pub fn maybe_uninit_conflict(&mut self, index: usize) -> &mut ReadoutConflict {
        &mut self.conflicts[index]
    }
}
