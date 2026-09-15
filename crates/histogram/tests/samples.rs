use nr_histogram::percentiles::PercentileError;
use nr_histogram::samples::stats_from_samples;

#[test]
fn stats_from_samples_rejects_an_empty_slice() {
    let result = stats_from_samples(&[], &[0.5]);
    assert!(matches!(result, Err(PercentileError::EmptySamples)));
}

#[test]
fn min_and_max_are_exact() {
    let stats = stats_from_samples(&[7, 3, 9001, 42], &[0.5]).expect("non-empty slice");
    assert_eq!(stats.min, 3);
    assert_eq!(stats.max, 9001);
}

#[test]
fn quantiles_come_back_in_the_order_requested() {
    let samples: Vec<u64> = (1..=1000).collect();
    let requested = [0.5, 0.99, 0.999];
    let stats = stats_from_samples(&samples, &requested).expect("non-empty slice");

    assert_eq!(stats.values.len(), requested.len());
    for (pair, &quantile) in stats.values.iter().zip(requested.iter()) {
        assert_eq!(pair.0, quantile);
    }
}

#[test]
fn count_is_the_slice_length() {
    let samples: Vec<u64> = (0..1000).collect();
    let stats = stats_from_samples(&samples, &[0.5]).expect("non-empty slice");
    assert_eq!(stats.count, 1000);
}

#[test]
fn a_single_sample_is_its_own_every_percentile() {
    let stats = stats_from_samples(&[500], &[0.5, 0.9, 0.99, 0.999]).expect("single-sample slice");
    assert_eq!(stats.min, 500);
    assert_eq!(stats.max, 500);
    for &(_, value) in &stats.values {
        assert_eq!(value, 500);
    }
}

#[test]
fn large_nanosecond_values_keep_five_significant_figures() {
    // 1000 distinct integer values clustered around 33_000, the project's 30 kHz poll period
    // in nanoseconds. The "true" order statistic is computed independently of hdrhistogram, by
    // sorting the same slice, so this test does not just check hdrhistogram against itself.
    let samples: Vec<u64> = (32_500..33_500).collect();
    let mut sorted = samples.clone();
    sorted.sort_unstable();
    let true_median = sorted[sorted.len() / 2 - 1];

    let stats = stats_from_samples(&samples, &[0.5]).expect("non-empty slice");
    let (_, computed) = stats.values[0];

    let relative_error = (computed as f64 - true_median as f64).abs() / true_median as f64;
    assert!(
        relative_error < 1.0 / 10_000.0,
        "expected within 1 part in 10_000 of {true_median}, got {computed}"
    );
}
