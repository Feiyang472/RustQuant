// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// RustQuant: A Rust library for quantitative finance tools.
// Copyright (C) 2023 https://github.com/avhz
// Dual licensed under Apache 2.0 and MIT.
// See:
//      - LICENSE-APACHE.md
//      - LICENSE-MIT.md
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

//! `Trajectories` is the return type of all the stochastic processes.
//! `StochasticProcess` is the base trait for all the stochastic processes.
//!
//! Currently only autonomous stochastic processes are implemented.
//! Autonomous refers to processes where the drift and diffusion
//! do not explicitly depend on the time `t`.

use rand::prelude::Distribution;
use rayon::prelude::*;

use crate::simulation::simulate_stochatic_process;

/// Struct to contain the time points and path values of the process.
pub struct Trajectories {
    /// Vector of time points.
    pub times: Vec<f64>,

    /// Vector of process trajectories.
    pub paths: Vec<Vec<f64>>,
}

/// Enum for Stochastic Methods
#[derive(Clone, Copy)]
pub enum StochasticScheme {
    /// Euler-Maruyama
    EulerMaruyama,
    /// Milstein's Method
    Milstein,
    /// Strang Splitting
    StrangSplitting,
}

/// Trait to implement stochastic volatility processes.
pub trait StochasticVolatilityProcess: Sync {
    /// Base method for the asset's drift.
    fn drift_1(&self, x: f64, t: f64) -> f64;

    /// Base method for the volatility process' drift.
    fn drift_2(&self, x: f64, t: f64) -> f64;

    /// Base method for the asset's diffusion.
    fn diffusion_1(&self, x: f64, t: f64) -> f64;

    /// Base method for the volatility process' diffusion.
    fn diffusion_2(&self, x: f64, t: f64) -> f64;

    /// Simulate via Euler-Maruyama discretisation scheme.
    fn euler_maruyama(
        &self,
        x_0: f64,
        y_0: f64,
        t_0: f64,
        t_n: f64,
        n_steps: usize,
        m_paths: usize,
        parallel: bool,
    ) -> Trajectories {
        assert!(t_0 < t_n);

        let dt: f64 = (t_n - t_0) / (n_steps as f64);

        // Initialise empty paths and fill in the time points.
        let mut x_paths = vec![vec![x_0; n_steps + 1]; m_paths];
        let mut y_paths = vec![vec![y_0; n_steps + 1]; m_paths];
        let times: Vec<f64> = (0..=n_steps).map(|t| t_0 + dt * (t as f64)).collect();

        let path_generator = |(x_path, y_path): (&mut Vec<f64>, &mut Vec<f64>)| {
            let mut rng = rand::thread_rng();
            let scale = dt.sqrt();
            let dW: Vec<f64> = rand_distr::Normal::new(0.0, 1.0)
                .unwrap()
                .sample_iter(&mut rng)
                .take(n_steps)
                .map(|z| z * scale)
                .collect();

            for t in 0..n_steps {
                x_path[t + 1] = x_path[t]
                    + self.drift_1(x_path[t], times[t]) * dt
                    + self.diffusion_1(x_path[t], times[t]) * dW[t];
                y_path[t + 1] = y_path[t]
                    + self.drift_2(y_path[t], times[t]) * dt
                    + self.diffusion_2(y_path[t], times[t]) * dW[t];
            }
        };

        if parallel {
            x_paths
                .par_iter_mut()
                .zip(y_paths.par_iter_mut())
                .for_each(path_generator);
        } else {
            x_paths
                .iter_mut()
                .zip(y_paths.iter_mut())
                .for_each(path_generator);
        }

        Trajectories {
            times: times.clone(),
            paths: x_paths,
        }
    }
}

/// Configuration parameters for simulating a stochastic process.
///
/// # Arguments:
/// * `x_0` - The process' initial value at `t_0`.
/// * `t_0` - The initial time point.
/// * `t_n` - The terminal time point.
/// * `n_steps` - The number of time steps between `t_0` and `t_n`.
/// * `m_paths` - How many process trajectories to simulate.
/// * `parallel` - Run in parallel or not (recommended for > 1000 paths).
pub struct StochasticProcessConfig {
    /// Initial value of the process.
    pub x_0: f64,

    /// Initial time point.
    pub t_0: f64,

    /// Terminal time point.
    pub t_n: f64,

    /// Number of time steps between `t_0` and `t_n`.
    pub n_steps: usize,

    /// Numerical method
    pub scheme: StochasticScheme,

    /// How many process trajectories to simulate.
    pub m_paths: usize,

    /// Run in parallel or not (recommended for > 1000 paths).
    pub parallel: bool,

    /// Optional seed argument to initialize random number generator
    pub seed: Option<u64>,
}

impl StochasticProcessConfig {
    /// Create a new configuration for a stochastic process.
    pub fn new(
        x_0: f64,
        t_0: f64,
        t_n: f64,
        n_steps: usize,
        scheme: StochasticScheme,
        m_paths: usize,
        parallel: bool,
        seed: Option<u64>,
    ) -> Self {
        Self {
            x_0,
            t_0,
            t_n,
            scheme,
            n_steps,
            m_paths,
            parallel,
            seed,
        }
    }

    pub(crate) fn unpack(
        &self,
    ) -> (
        f64,
        f64,
        f64,
        usize,
        StochasticScheme,
        usize,
        bool,
        Option<u64>,
    ) {
        (
            self.x_0,
            self.t_0,
            self.t_n,
            self.n_steps,
            self.scheme,
            self.m_paths,
            self.parallel,
            self.seed,
        )
    }

    fn unpack_for_scheme(&self, dt: f64, config: &StochasticProcessConfig) -> (f64, usize, Vec<f64>) {
        let times: Vec<f64> = (0..=config.n_steps)
        .map(|t| config.t_0 + dt * (t as f64))
        .collect();

        (self.x_0, self.n_steps, times)
    }
}

/// Trait to implement stochastic processes.
#[allow(clippy::module_name_repetitions)]
pub trait StochasticProcess: Sync {
    /// Base method for the process' drift.
    fn drift(&self, x: f64, t: f64) -> f64;

    /// Base method for the process' diffusion.
    fn diffusion(&self, x: f64, t: f64) -> f64;

    /// Base method for the process' jump term (if applicable).
    fn jump(&self, x: f64, t: f64) -> Option<f64>;

    /// Return the model's parameters as a `Vec<f64>`.
    fn parameters(&self) -> Vec<f64> {
        vec![]
    }

    /// Simulate the stochastic process.
    fn generate(&self, config: &StochasticProcessConfig) -> Trajectories
    where
        Self: Sized,
    {
        simulate_stochatic_process(self, config, None, None)
    }
}

#[cfg(test)]
mod test_process {
    use crate::geometric_brownian_motion::GeometricBrownianMotion;
    use crate::{StochasticScheme, StochasticProcessConfig, StochasticProcess};
    use std::time::Instant;

    #[test]
    fn test_euler_maruyama() {
        let gbm = GeometricBrownianMotion::new(0.05, 0.9);
        let config = StochasticProcessConfig::new(
            10.0,
            0.0,
            1.0,
            10,
            StochasticScheme::EulerMaruyama,
            3,
            false,
            Some(1337),
        );

        let start = Instant::now();
        gbm.generate(&config);
        let serial = start.elapsed();

        println!("Serial: \t {:?}", serial);

        let start = Instant::now();
        gbm.generate(&config);
        let parallel = start.elapsed();

        println!("Parallel: \t {:?}", parallel);

        // Just checking that `parallel = true` actually works.
        // To see the output of this "test", run:
        // cargo test test_process -- --nocapture
    }

    #[test]
    fn test_milstein() {
        let gbm = GeometricBrownianMotion::new(0.05, 0.9);
        let config = StochasticProcessConfig::new(
            10.0,
            0.0,
            1.0,
            10,
            StochasticScheme::Milstein,
            3,
            false,
            Some(1337),
        );

        let start = Instant::now();
        gbm.generate(&config);
        let serial = start.elapsed();

        println!("Serial: \t {:?}", serial);

        let start = Instant::now();
        gbm.generate(&config);
        let parallel = start.elapsed();

        println!("Parallel: \t {:?}", parallel);

        // Just checking that `parallel = true` actually works.
        // To see the output of this "test", run:
        // cargo test test_process -- --nocapture
    }

    #[test]
    fn test_strang_splitter() {
        let gbm = GeometricBrownianMotion::new(0.05, 0.9);
        let config = StochasticProcessConfig::new(
            10.0,
            0.0,
            1.0,
            10,
            StochasticScheme::StrangSplitting,
            3,
            false,
            Some(1337),
        );

        let start = Instant::now();
        gbm.generate(&config);
        let serial = start.elapsed();

        println!("Serial: \t {:?}", serial);

        let start = Instant::now();
        gbm.generate(&config);
        let parallel = start.elapsed();

        println!("Parallel: \t {:?}", parallel);

        // Just checking that `parallel = true` actually works.
        // To see the output of this "test", run:
        // cargo test test_process -- --nocapture
    }
}
