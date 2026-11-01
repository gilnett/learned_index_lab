// Copyright (c) Learned Index
// Dataset generation for synthetic and SOSD distribution workloads

/// Minimal PRNG (Xorshift64) for reproducible dataset and query generation.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributionType {
    DenseTimestamp,
    UniformSparse,
    ClusteredLognormal,
    SosdOsmCellIds,
    SosdWikiTs,
    SosdBooks,
}

impl DistributionType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DenseTimestamp => "Dense Sequential / Time-Series",
            Self::UniformSparse => "Uniform Sparse Random Distribution",
            Self::ClusteredLognormal => "Heavy-Tailed Lognormal Distribution",
            Self::SosdOsmCellIds => "SOSD OpenStreetMap (OSM CellIDs)",
            Self::SosdWikiTs => "SOSD Wikipedia Timestamps (Wiki-TS)",
            Self::SosdBooks => "SOSD Books (Heavy-Tailed Book ISBNs)",
        }
    }

    pub fn default_epsilon(&self) -> usize {
        match self {
            Self::DenseTimestamp => 32,
            Self::UniformSparse => 16,
            Self::ClusteredLognormal => 16,
            Self::SosdOsmCellIds => 16,
            Self::SosdWikiTs => 32,
            Self::SosdBooks => 16,
        }
    }
}

pub fn generate_dataset(dist: DistributionType, count: usize, seed: u64) -> Vec<u64> {
    let mut rng = Rng::new(seed);
    let mut data = Vec::with_capacity(count);

    match dist {
        DistributionType::DenseTimestamp => {
            let mut current = 1_700_000_000u64;
            for _ in 0..count {
                current += 1 + (rng.next_u64() % 5);
                data.push(current);
            }
        }
        DistributionType::UniformSparse => {
            let mut set = std::collections::BTreeSet::new();
            while set.len() < count {
                let v = rng.next_u64() % 1_000_000_000_000;
                set.insert(v);
            }
            data.extend(set);
        }
        DistributionType::ClusteredLognormal => {
            let mut raw = Vec::with_capacity(count);
            for _ in 0..count {
                let u1 = rng.next_f64().max(1e-9);
                let u2 = rng.next_f64().max(1e-9);
                let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let lognormal = (z0 * 1.5 + 10.0).exp();
                raw.push(lognormal as u64);
            }
            raw.sort_unstable();
            raw.dedup();
            let mut current = raw.last().copied().unwrap_or(0);
            while raw.len() < count {
                current += 1 + (rng.next_u64() % 10);
                raw.push(current);
            }
            data = raw;
        }
        DistributionType::SosdOsmCellIds => {
            let num_clusters = (count / 250).max(10);
            let mut cluster_centers = Vec::with_capacity(num_clusters);
            for _ in 0..num_clusters {
                cluster_centers.push(rng.next_u64() % 500_000_000_000);
            }
            let mut raw = Vec::with_capacity(count);
            for i in 0..count {
                let center = cluster_centers[i % num_clusters];
                let offset = rng.next_u64() % 500_000;
                raw.push(center.wrapping_add(offset));
            }
            raw.sort_unstable();
            raw.dedup();
            let mut current = raw.last().copied().unwrap_or(0);
            while raw.len() < count {
                current += 1 + (rng.next_u64() % 30);
                raw.push(current);
            }
            data = raw;
        }
        DistributionType::SosdWikiTs => {
            let mut raw = Vec::with_capacity(count);
            let mut current_ts = 1_000_000_000u64;
            while raw.len() < count {
                let burst_len = (1 + (rng.next_u64() % 12)) as usize;
                let step = rng.next_u64() % 3;
                for _ in 0..burst_len {
                    if raw.len() < count {
                        raw.push(current_ts);
                    }
                }
                current_ts += step;
                if rng.next_u64().is_multiple_of(25) {
                    current_ts += 50 + (rng.next_u64() % 300);
                }
            }
            data = raw;
        }
        DistributionType::SosdBooks => {
            let mut raw = Vec::with_capacity(count);
            for _ in 0..count {
                let u = rng.next_f64().max(1e-7);
                let pareto = 100.0 / (u.powf(1.0 / 1.3));
                raw.push((pareto as u64).min(50_000_000_000));
            }
            raw.sort_unstable();
            let mut current = raw.last().copied().unwrap_or(0);
            while raw.len() < count {
                current += 1 + (rng.next_u64() % 15);
                raw.push(current);
            }
            data = raw;
        }
    }

    data
}

pub fn generate_internal_miss_queries(dataset: &[u64], count: usize, rng: &mut Rng) -> Vec<u64> {
    let mut queries = Vec::with_capacity(count);
    if dataset.len() < 2 {
        return queries;
    }

    let min_k = dataset[0];
    let max_k = *dataset.last().expect("invariant: non-empty dataset");
    let span = max_k.saturating_sub(min_k);

    if span == 0 {
        return queries;
    }

    let mut attempts = 0;
    while queries.len() < count && attempts < count * 20 {
        attempts += 1;
        let idx = (rng.next_u64() as usize) % (dataset.len() - 1);
        let left = dataset[idx];
        let right = dataset[idx + 1];
        if right > left + 1 {
            let gap = right - left - 1;
            let offset = 1 + (rng.next_u64() % gap);
            queries.push(left + offset);
        }
    }

    while queries.len() < count {
        let candidate = min_k + (rng.next_u64() % span);
        if dataset.binary_search(&candidate).is_err() {
            queries.push(candidate);
        }
    }

    queries
}
