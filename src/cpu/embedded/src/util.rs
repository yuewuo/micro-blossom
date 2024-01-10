use crate::binding::*;
use core::hint::black_box;
use core::sync::atomic::{compiler_fence, Ordering};

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
        }
    }

    /// run a batch of benchmarker function, so that each batch takes at least 1s
    /// (but no more than 10s to avoid timer overflow)
    pub fn autotune(&mut self) {
        println!("[benchmarker] autotune");
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
            if diff >= 0.1 {
                batch_size = core::cmp::max(1, (1.0 / diff * (batch_size as f64)) as usize);
                break;
            }
            batch_size *= 2;
        }
        self.batch_size = batch_size;
        println!("[benchmarker] automatic batch size = {batch_size}");
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
