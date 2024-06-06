//! Latency Benchmarker
//!
//! Recording the latency distribution. There are N buckets. Each bucket counts the number of samples
//! whose log value is within certain range. This is useful to plot a log-log plot where the x axis
//! is the log value of the latency and the y axis is log value of the probability
//!

#[allow(unused_imports)]
use crate::util::*;
use libm::{floor, log, pow};

pub struct LatencyBenchmarker<const N: usize = 2000> {
    pub lower: f64,
    pub upper: f64,
    pub counter: [usize; N],
    /// the number of samples below `lower`
    pub underflow_count: usize,
    /// the number of samples above `upper`
    pub overflow_count: usize,
}

impl<const N: usize> LatencyBenchmarker<N> {
    pub const fn new(lower: f64, upper: f64) -> Self {
        assert!(lower > 0.);
        assert!(upper > lower);
        Self {
            lower,
            upper,
            counter: [0; N],
            underflow_count: 0,
            overflow_count: 0,
        }
    }

    /// by default starting from 1ns to 1s, with resolution of 1% difference
    /// because log(1e9, 1.01) ~ 2000
    pub const fn new_default() -> Self {
        Self::new(1e-9, 1.)
    }

    pub fn clear(&mut self) {
        for i in 0..N {
            self.counter[i] = 0;
        }
        self.underflow_count = 0;
        self.overflow_count = 0;
    }

    pub fn record(&mut self, latency: f64) {
        if latency < self.lower {
            self.underflow_count += 1;
        } else if latency >= self.upper {
            self.overflow_count += 1;
        } else {
            let ratio = log(latency / self.lower) / log(self.upper / self.lower);
            let index = floor((N as f64) * ratio) as usize;
            assert!(index < N);
            self.counter[index] += 1;
        }
    }

    pub fn iter_nonzero(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..N)
            .filter(move |index| self.counter[*index] > 0)
            .map(move |index| (index, self.counter[index]))
    }

    pub fn println(&self) {
        print!("<lower>{:.3e}<upper>{:.3e}<N>{}", self.lower, self.upper, N);
        for (index, counter) in self.iter_nonzero() {
            print!("[{index}]{counter}");
        }
        println!("[underflow]{}[overflow]{}", self.underflow_count, self.overflow_count);
    }

    pub fn latency_of(&self, index: usize) -> f64 {
        self.lower * pow(self.upper / self.lower, (index as f64 + 0.5) / (N as f64))
    }

    pub fn debug_println(&self) {
        println!("lower: {:.3e}s, upper: {:.3e}s, N: {}", self.lower, self.upper, N);
        for (index, counter) in self.iter_nonzero() {
            println!("    [{index}] {counter} ( ~ {:.3e}s )", self.latency_of(index));
        }
        println!("    [underflow] {} ( < {:.3e}s )", self.underflow_count, self.lower);
        println!("    [overflow] {} ( >= {:.3e}s )", self.overflow_count, self.upper);
    }

    pub fn count_records(&self) -> usize {
        self.iter_nonzero().map(|(_index, count)| count).sum()
    }

    pub fn average_latency(&self) -> f64 {
        let sum_latency: f64 = self
            .iter_nonzero()
            .map(|(index, count)| (count as f64) * self.latency_of(index))
            .sum();
        sum_latency / (self.count_records() as f64)
    }

    pub fn percentile_latency_index(&self, percentile: f64) -> usize {
        let num_records = self.count_records();
        let mut sum_counter = 0;
        for (index, counter) in self.iter_nonzero() {
            sum_counter += counter;
            let current_percent = (sum_counter as f64) / (num_records as f64);
            if current_percent >= percentile {
                return index;
            }
        }
        N - 1
    }

    /// print useful statistics like average, 80 percentile, 90, 99, 99.9 percentile, etc.
    pub fn print_statistics(&self) {
        if self.underflow_count > 0 {
            println!("[warning] statistics may not be accurate because of underflow");
        }
        if self.overflow_count > 0 {
            println!("[warning] statistics may not be accurate because of overflow");
        }
        println!("average latency: {:.3e}s", self.average_latency());
        for percentile in [0.8, 0.9, 0.99, 0.999] {
            let index = self.percentile_latency_index(percentile);
            println!("{percentile} percentile: [{index}] ( ~ {:.3e}s )", self.latency_of(index));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_benchmarker_basic() {
        // cargo test latency_benchmarker_basic -- --nocapture
        let mut benchmarker: LatencyBenchmarker = LatencyBenchmarker::new_default();
        let mut record_multiple = |latency: f64, count: usize| {
            for _ in 0..count {
                benchmarker.record(latency);
            }
        };
        record_multiple(1e-9, 10);
        record_multiple(1e-6, 3);
        record_multiple(1.5e-6, 6);
        record_multiple(3e-3, 2);
        record_multiple(0.99999, 1);
        // print out the result
        benchmarker.debug_println();
        benchmarker.println();
    }
}
