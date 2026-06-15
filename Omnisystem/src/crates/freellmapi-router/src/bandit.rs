use rand::Rng;

/// Beta distribution for Thompson Sampling
#[derive(Debug, Clone)]
pub struct BetaDistribution {
    pub alpha: f64, // successes + 1
    pub beta: f64,  // failures + 1
}

impl BetaDistribution {
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }

    /// Sample from Beta distribution using Gamma function
    pub fn sample(&self) -> f64 {
        let mut rng = rand::thread_rng();
        sample_beta(self.alpha, self.beta, &mut rng)
    }

    /// Update with feedback
    pub fn record_success(&mut self) {
        self.alpha += 1.0;
    }

    pub fn record_failure(&mut self) {
        self.beta += 1.0;
    }

    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }
}

/// Sample from Beta(alpha, beta) distribution using Gamma distribution
pub fn sample_beta<R: Rng>(alpha: f64, beta: f64, rng: &mut R) -> f64 {
    let g_alpha = sample_gamma(alpha, rng);
    let g_beta = sample_gamma(beta, rng);
    g_alpha / (g_alpha + g_beta)
}

/// Sample from Gamma distribution using Marsaglia & Tsang method
fn sample_gamma<R: Rng>(shape: f64, rng: &mut R) -> f64 {
    if shape < 1.0 {
        let u: f64 = rng.gen();
        return sample_gamma(shape + 1.0, rng) * u.powf(1.0 / shape);
    }

    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();

    loop {
        let x: f64 = rng.gen();
        if x < -1.0 {
            continue;
        }

        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 {
            continue;
        }

        let u: f64 = rng.gen();
        let x_sq = x * x;

        if u < 1.0 - 0.0331 * x_sq * x_sq {
            return d * v;
        }

        if u.ln() < 0.5 * x_sq + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_distribution_creation() {
        let beta = BetaDistribution::new(1.0, 1.0);
        assert_eq!(beta.alpha, 1.0);
        assert_eq!(beta.beta, 1.0);
    }

    #[test]
    fn test_beta_mean() {
        let beta = BetaDistribution::new(2.0, 2.0);
        assert!((beta.mean() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_beta_sampling() {
        let beta = BetaDistribution::new(10.0, 10.0);
        let sample = beta.sample();
        assert!(sample >= 0.0 && sample <= 1.0);
    }

    #[test]
    fn test_feedback_recording() {
        let mut beta = BetaDistribution::new(1.0, 1.0);
        beta.record_success();
        assert_eq!(beta.alpha, 2.0);

        beta.record_failure();
        assert_eq!(beta.beta, 2.0);
    }

    #[test]
    fn test_sample_beta_distribution() {
        let mut rng = rand::thread_rng();
        let sample = sample_beta(2.0, 2.0, &mut rng);
        assert!(sample >= 0.0 && sample <= 1.0);
    }

    #[test]
    fn test_biased_beta() {
        let mut rng = rand::thread_rng();
        // Beta(10, 1) is biased towards 1
        let samples: Vec<f64> = (0..100)
            .map(|_| sample_beta(10.0, 1.0, &mut rng))
            .collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(mean > 0.8); // Should be biased high
    }
}
