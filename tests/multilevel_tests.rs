use learned_index_lab::{MultilevelPgmIndex, PgmIndex};

#[test]
fn test_multilevel_parity_with_flat_pgm_and_binary_search() {
    // 20,000 keys with variable density and step gaps
    let mut data = Vec::with_capacity(20_000);
    let mut current = 100u64;
    for i in 0..20_000 {
        if i % 100 == 0 {
            current += 500; // gap
        } else if i % 10 == 0 {
            current += 0; // duplicate
        } else {
            current += 3;
        }
        data.push(current);
    }

    let flat_pgm = PgmIndex::build(data.clone(), 8);
    let multi_pgm = MultilevelPgmIndex::build_with_epsilons(data.clone(), 8, 4);

    assert!(
        multi_pgm.height() >= 2,
        "Multilevel PGM must construct at least 2 levels for 20k elements"
    );

    // Verify find parity for sample queries across the domain
    for &key in data.iter().step_by(23) {
        let expected_flat = flat_pgm.find(key);
        let expected_bin = data.binary_search(&key).ok();
        let actual_multi = multi_pgm.find(key);

        assert!(actual_multi.is_some());
        assert_eq!(data[actual_multi.unwrap()], key);
        assert_eq!(data[expected_flat.unwrap()], key);
        assert_eq!(data[expected_bin.unwrap()], key);
    }

    // Verify non-existent keys
    for key in [0, 50, current + 1, current + 1000] {
        assert_eq!(multi_pgm.find(key), None);
    }
}

#[test]
fn test_multilevel_deep_hierarchy_with_tiny_epsilon() {
    let data: Vec<u64> = (0..50_000).map(|x| (x * x) / 50 + x).collect();
    let multi_pgm = MultilevelPgmIndex::build_with_epsilons(data.clone(), 4, 2);

    assert!(
        multi_pgm.height() >= 3,
        "Tree height should be >= 3 for 50k non-linear elements with epsilon_rec=2"
    );

    for &key in data.iter().step_by(100) {
        let found = multi_pgm.find(key).expect("Key should be found");
        assert_eq!(data[found], key);
    }
}
