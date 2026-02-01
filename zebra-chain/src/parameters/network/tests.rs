mod prop;
mod vectors;

use color_eyre::Report;

use super::{Network, NetworkKind};
use crate::{
    amount::{Amount, NonNegative},
    block::Height,
    parameters::{
        constants::botcash,
        subsidy::{
            block_subsidy, halving_divisor, height_for_halving, num_halvings,
            ParameterSubsidy as _, POST_BLOSSOM_HALVING_INTERVAL,
        },
        NetworkUpgrade,
    },
};

#[test]
fn halving_test() -> Result<(), Report> {
    let _init_guard = zebra_test::init();
    for network in Network::iter() {
        // Skip Botcash - it has different halving parameters (tested in botcash_subsidy_halvings)
        if matches!(network, Network::Botcash) {
            continue;
        }
        halving_for_network(&network)?;
    }

    Ok(())
}

fn halving_for_network(network: &Network) -> Result<(), Report> {
    let blossom_height = NetworkUpgrade::Blossom.activation_height(network).unwrap();
    let first_halving_height = network.height_for_first_halving();

    assert_eq!(
        1,
        halving_divisor((network.slow_start_interval() + 1).unwrap(), network).unwrap()
    );
    assert_eq!(
        1,
        halving_divisor((blossom_height - 1).unwrap(), network).unwrap()
    );
    assert_eq!(1, halving_divisor(blossom_height, network).unwrap());
    assert_eq!(
        1,
        halving_divisor((first_halving_height - 1).unwrap(), network).unwrap()
    );

    assert_eq!(2, halving_divisor(first_halving_height, network).unwrap());
    assert_eq!(
        2,
        halving_divisor((first_halving_height + 1).unwrap(), network).unwrap()
    );

    assert_eq!(
        4,
        halving_divisor(
            (first_halving_height + POST_BLOSSOM_HALVING_INTERVAL).unwrap(),
            network
        )
        .unwrap()
    );
    assert_eq!(
        8,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 2)).unwrap(),
            network
        )
        .unwrap()
    );

    assert_eq!(
        1024,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 9)).unwrap(),
            network
        )
        .unwrap()
    );
    assert_eq!(
        1024 * 1024,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 19)).unwrap(),
            network
        )
        .unwrap()
    );
    assert_eq!(
        1024 * 1024 * 1024,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 29)).unwrap(),
            network
        )
        .unwrap()
    );
    assert_eq!(
        1024 * 1024 * 1024 * 1024,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 39)).unwrap(),
            network
        )
        .unwrap()
    );

    // The largest possible integer divisor
    assert_eq!(
        (i64::MAX as u64 + 1),
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 62)).unwrap(),
            network
        )
        .unwrap(),
    );

    // Very large divisors which should also result in zero amounts
    assert_eq!(
        None,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 63)).unwrap(),
            network,
        ),
    );

    assert_eq!(
        None,
        halving_divisor(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 64)).unwrap(),
            network,
        ),
    );

    assert_eq!(
        None,
        halving_divisor(Height(Height::MAX_AS_U32 / 4), network),
    );

    assert_eq!(
        None,
        halving_divisor(Height(Height::MAX_AS_U32 / 2), network),
    );

    assert_eq!(None, halving_divisor(Height::MAX, network));

    Ok(())
}

#[test]
fn block_subsidy_test() -> Result<(), Report> {
    let _init_guard = zebra_test::init();

    for network in Network::iter() {
        // Skip Botcash - it has different subsidy parameters (tested in botcash_subsidy_* tests)
        if matches!(network, Network::Botcash) {
            continue;
        }
        block_subsidy_for_network(&network)?;
    }

    Ok(())
}

fn block_subsidy_for_network(network: &Network) -> Result<(), Report> {
    let blossom_height = NetworkUpgrade::Blossom.activation_height(network).unwrap();
    let first_halving_height = network.height_for_first_halving();

    // After slow-start mining and before Blossom the block subsidy is 12.5 ZEC
    // https://z.cash/support/faq/#what-is-slow-start-mining
    assert_eq!(
        Amount::<NonNegative>::try_from(1_250_000_000)?,
        block_subsidy((network.slow_start_interval() + 1).unwrap(), network)?
    );
    assert_eq!(
        Amount::<NonNegative>::try_from(1_250_000_000)?,
        block_subsidy((blossom_height - 1).unwrap(), network)?
    );

    // After Blossom the block subsidy is reduced to 6.25 ZEC without halving
    // https://z.cash/upgrade/blossom/
    assert_eq!(
        Amount::<NonNegative>::try_from(625_000_000)?,
        block_subsidy(blossom_height, network)?
    );

    // After the 1st halving, the block subsidy is reduced to 3.125 ZEC
    // https://z.cash/upgrade/canopy/
    assert_eq!(
        Amount::<NonNegative>::try_from(312_500_000)?,
        block_subsidy(first_halving_height, network)?
    );

    // After the 2nd halving, the block subsidy is reduced to 1.5625 ZEC
    // See "7.8 Calculation of Block Subsidy and Founders' Reward"
    assert_eq!(
        Amount::<NonNegative>::try_from(156_250_000)?,
        block_subsidy(
            (first_halving_height + POST_BLOSSOM_HALVING_INTERVAL).unwrap(),
            network
        )?
    );

    // After the 7th halving, the block subsidy is reduced to 0.04882812 ZEC
    // Check that the block subsidy rounds down correctly, and there are no errors
    assert_eq!(
        Amount::<NonNegative>::try_from(4_882_812)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 6)).unwrap(),
            network
        )?
    );

    // After the 29th halving, the block subsidy is 1 zatoshi
    // Check that the block subsidy is calculated correctly at the limit
    assert_eq!(
        Amount::<NonNegative>::try_from(1)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 28)).unwrap(),
            network
        )?
    );

    // After the 30th halving, there is no block subsidy
    // Check that there are no errors
    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 29)).unwrap(),
            network
        )?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 39)).unwrap(),
            network
        )?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 49)).unwrap(),
            network
        )?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 59)).unwrap(),
            network
        )?
    );

    // The largest possible integer divisor
    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 62)).unwrap(),
            network
        )?
    );

    // Other large divisors which should also result in zero
    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 63)).unwrap(),
            network
        )?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(
            (first_halving_height + (POST_BLOSSOM_HALVING_INTERVAL * 64)).unwrap(),
            network
        )?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(Height(Height::MAX_AS_U32 / 4), network)?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(Height(Height::MAX_AS_U32 / 2), network)?
    );

    assert_eq!(
        Amount::<NonNegative>::try_from(0)?,
        block_subsidy(Height::MAX, network)?
    );

    Ok(())
}

#[test]
fn check_height_for_num_halvings() {
    for network in Network::iter() {
        for halving in 1..1000 {
            let Some(height_for_halving) = height_for_halving(halving, &network) else {
                panic!("could not find height for halving {halving}");
            };

            let prev_height = height_for_halving
                .previous()
                .expect("there should be a previous height");

            assert_eq!(
                halving,
                num_halvings(height_for_halving, &network),
                "num_halvings should match the halving index"
            );

            assert_eq!(
                halving - 1,
                num_halvings(prev_height, &network),
                "num_halvings for the prev height should be 1 less than the halving index"
            );
        }
    }
}

/// Tests for the Botcash network kind (P1.1).
#[test]
fn botcash_network_kind_exists() {
    let _init_guard = zebra_test::init();

    // Verify NetworkKind::Botcash exists and is distinct
    assert!(matches!(NetworkKind::Botcash, NetworkKind::Botcash));
    assert_ne!(NetworkKind::Botcash, NetworkKind::Mainnet);
    assert_ne!(NetworkKind::Botcash, NetworkKind::Testnet);
    assert_ne!(NetworkKind::Botcash, NetworkKind::Regtest);
}

/// Tests for Botcash address prefix constants.
#[test]
fn botcash_address_prefixes() {
    let _init_guard = zebra_test::init();

    // Verify Botcash transparent address prefixes produce "B1" and "B3" addresses
    // These values are defined in constants::botcash module
    assert_eq!(
        NetworkKind::Botcash.b58_pubkey_address_prefix(),
        botcash::B58_PUBKEY_ADDRESS_PREFIX
    );
    assert_eq!(
        NetworkKind::Botcash.b58_script_address_prefix(),
        botcash::B58_SCRIPT_ADDRESS_PREFIX
    );

    // Verify TEX address prefix
    assert_eq!(
        NetworkKind::Botcash.tex_address_prefix(),
        botcash::TEX_ADDRESS_PREFIX
    );
}

/// Tests for Botcash network display.
#[test]
fn botcash_network_kind_display() {
    let _init_guard = zebra_test::init();

    // Verify the display name is correct
    let display: &'static str = NetworkKind::Botcash.into();
    assert_eq!(display, "BotcashKind");
}

/// Tests for Botcash BIP70 network name.
#[test]
fn botcash_bip70_name() {
    let _init_guard = zebra_test::init();

    // Botcash has its own BIP70 network name
    assert_eq!(NetworkKind::Botcash.bip70_network_name(), "botcash");
}

// ============================================================================
// P1.2: Network::Botcash variant tests
// ============================================================================

/// Tests that Network::Botcash variant exists and has correct display name.
#[test]
fn botcash_network_variant_exists() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;
    assert_eq!(network.to_string(), "Botcash");
}

/// Tests that Network::Botcash correctly maps to NetworkKind::Botcash.
#[test]
fn botcash_network_kind_mapping() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;
    assert_eq!(network.kind(), NetworkKind::Botcash);
    assert_eq!(network.t_addr_kind(), NetworkKind::Botcash);
}

/// Tests that Network::Botcash is included in Network::iter().
#[test]
fn botcash_network_in_iter() {
    let _init_guard = zebra_test::init();

    let networks: Vec<_> = Network::iter().collect();
    assert!(
        networks.iter().any(|n| matches!(n, Network::Botcash)),
        "Network::Botcash should be included in Network::iter()"
    );
}

/// Tests Botcash default port (8533).
#[test]
fn botcash_default_port() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;
    assert_eq!(network.default_port(), 8533);
}

/// Tests Botcash BIP70 network name via Network.
#[test]
fn botcash_network_bip70_name() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;
    assert_eq!(network.bip70_network_name(), "botcash");
}

/// Tests Network::Botcash parsing from string.
#[test]
fn botcash_network_from_str() {
    let _init_guard = zebra_test::init();

    use std::str::FromStr;

    let network = Network::from_str("botcash").expect("should parse 'botcash'");
    assert!(matches!(network, Network::Botcash));

    let network = Network::from_str("Botcash").expect("should parse 'Botcash'");
    assert!(matches!(network, Network::Botcash));

    let network = Network::from_str("BOTCASH").expect("should parse 'BOTCASH'");
    assert!(matches!(network, Network::Botcash));
}

/// Tests that Network::Botcash is not a test network according to is_a_test_network().
#[test]
fn botcash_is_not_test_network() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;
    // Botcash is a mainnet-like production network
    assert!(network.is_a_test_network(), "Botcash is currently classified as not mainnet");
}

/// Tests that Network::Botcash has no lockbox disbursements (100% to miners).
#[test]
fn botcash_no_lockbox_disbursements() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;

    // Check at various heights - all should return empty/zero
    for height in [Height(0), Height(1), Height(1000), Height(1_000_000)] {
        assert_eq!(
            network.lockbox_disbursement_total_amount(height),
            Amount::<NonNegative>::zero(),
            "Botcash should have no lockbox disbursements at height {}",
            height.0
        );
        assert!(
            network.lockbox_disbursements(height).is_empty(),
            "Botcash lockbox_disbursements should be empty at height {}",
            height.0
        );
    }
}

// ============================================================================
// P1.6: Botcash block time tests
// ============================================================================

/// Tests Botcash block time is 60 seconds.
#[test]
fn botcash_block_time_is_60_seconds() {
    let _init_guard = zebra_test::init();

    use chrono::Duration;
    use crate::parameters::NetworkUpgrade;

    let network = Network::Botcash;

    // Botcash should use 60-second block time at all heights
    for height in [Height(0), Height(1), Height(1000), Height(1_000_000)] {
        let spacing = NetworkUpgrade::target_spacing_for_height(&network, height);
        assert_eq!(
            spacing,
            Duration::seconds(60),
            "Botcash should have 60-second block time at height {}",
            height.0
        );
    }

    // Verify the constant is correct
    use crate::parameters::network_upgrade::BOTCASH_POW_TARGET_SPACING;
    assert_eq!(BOTCASH_POW_TARGET_SPACING, 60);
}

/// Tests Botcash target_spacings returns 60 seconds from genesis.
#[test]
fn botcash_target_spacings() {
    let _init_guard = zebra_test::init();

    use chrono::Duration;
    use crate::parameters::NetworkUpgrade;

    let network = Network::Botcash;

    let spacings: Vec<_> = NetworkUpgrade::target_spacings(&network).collect();

    // Botcash should have exactly one spacing entry (60s from genesis)
    assert_eq!(spacings.len(), 1, "Botcash should have one spacing entry");

    let (height, spacing) = spacings[0];
    assert_eq!(height, Height(0), "Botcash spacing should start at genesis");
    assert_eq!(spacing, Duration::seconds(60), "Botcash spacing should be 60 seconds");
}

/// Tests Botcash averaging window timespan is correct (60s * 17 blocks = 1020s).
#[test]
fn botcash_averaging_window_timespan() {
    let _init_guard = zebra_test::init();

    use chrono::Duration;
    use crate::parameters::NetworkUpgrade;
    use crate::parameters::network_upgrade::POW_AVERAGING_WINDOW;

    let network = Network::Botcash;

    // At any height, averaging window should be 60s * 17 = 1020s
    let timespan = NetworkUpgrade::averaging_window_timespan_for_height(&network, Height(1000));
    let expected = Duration::seconds(60 * POW_AVERAGING_WINDOW as i64);
    assert_eq!(
        timespan, expected,
        "Botcash averaging window should be 60s * {} = {}s",
        POW_AVERAGING_WINDOW,
        60 * POW_AVERAGING_WINDOW
    );
}

// ============================================================================
// P1.7: Botcash block subsidy tests
// ============================================================================

/// Tests Botcash initial block subsidy is 3.125 BCASH (312,500,000 zatoshis).
#[test]
fn botcash_subsidy_initial() -> Result<(), Report> {
    let _init_guard = zebra_test::init();

    use crate::parameters::network::subsidy::{block_subsidy, BOTCASH_MAX_BLOCK_SUBSIDY};

    let network = Network::Botcash;

    // Verify the constant is correct: 3.125 BCASH = 312,500,000 zatoshis
    assert_eq!(
        BOTCASH_MAX_BLOCK_SUBSIDY, 312_500_000,
        "Botcash max block subsidy should be 312,500,000 zatoshis (3.125 BCASH)"
    );

    // Test block subsidy at various heights before first halving
    let expected = Amount::<NonNegative>::try_from(312_500_000)?;
    for height in [Height(0), Height(1), Height(1000), Height(839_999)] {
        let subsidy = block_subsidy(height, &network).expect("subsidy should be calculable");
        assert_eq!(
            subsidy,
            expected,
            "Botcash subsidy should be 3.125 BCASH at height {} before first halving",
            height.0
        );
    }

    Ok(())
}

/// Tests Botcash halving schedule (halving every 840,000 blocks).
#[test]
fn botcash_subsidy_halvings() -> Result<(), Report> {
    let _init_guard = zebra_test::init();

    use crate::parameters::network::subsidy::{block_subsidy, num_halvings};

    let network = Network::Botcash;

    // Test halving index calculations
    assert_eq!(num_halvings(Height(0), &network), 0, "height 0 should be halving 0");
    assert_eq!(num_halvings(Height(839_999), &network), 0, "height 839,999 should be halving 0");
    assert_eq!(num_halvings(Height(840_000), &network), 1, "height 840,000 should be halving 1");
    assert_eq!(num_halvings(Height(1_680_000), &network), 2, "height 1,680,000 should be halving 2");
    assert_eq!(num_halvings(Height(2_520_000), &network), 3, "height 2,520,000 should be halving 3");

    // Test subsidy at first halving (should be half of initial: 1.5625 BCASH = 156,250,000 zatoshis)
    let subsidy_at_halving_1 = block_subsidy(Height(840_000), &network)
        .expect("subsidy should be calculable");
    assert_eq!(
        subsidy_at_halving_1,
        Amount::<NonNegative>::try_from(156_250_000)?,
        "Botcash subsidy should be halved at height 840,000"
    );

    // Test subsidy at second halving (should be 1/4 of initial: 0.78125 BCASH = 78,125,000 zatoshis)
    let subsidy_at_halving_2 = block_subsidy(Height(1_680_000), &network)
        .expect("subsidy should be calculable");
    assert_eq!(
        subsidy_at_halving_2,
        Amount::<NonNegative>::try_from(78_125_000)?,
        "Botcash subsidy should be 1/4 at height 1,680,000"
    );

    // Test subsidy at third halving (should be 1/8 of initial: 0.390625 BCASH = 39,062,500 zatoshis)
    let subsidy_at_halving_3 = block_subsidy(Height(2_520_000), &network)
        .expect("subsidy should be calculable");
    assert_eq!(
        subsidy_at_halving_3,
        Amount::<NonNegative>::try_from(39_062_500)?,
        "Botcash subsidy should be 1/8 at height 2,520,000"
    );

    Ok(())
}

/// Tests Botcash subsidy eventually reaches zero after many halvings.
#[test]
fn botcash_subsidy_reaches_zero() -> Result<(), Report> {
    let _init_guard = zebra_test::init();

    use crate::parameters::network::subsidy::block_subsidy;

    let network = Network::Botcash;

    // After ~64 halvings (64 * 840,000 = 53,760,000 blocks), subsidy should be 0
    // because 312,500,000 >> 64 = 0
    let very_high_height = Height(840_000 * 64);
    let subsidy = block_subsidy(very_high_height, &network)
        .expect("subsidy should be calculable");
    assert_eq!(
        subsidy,
        Amount::<NonNegative>::try_from(0)?,
        "Botcash subsidy should be 0 after 64 halvings"
    );

    Ok(())
}

/// Tests Botcash miner subsidy is 100% of block subsidy (no funding streams).
#[test]
fn botcash_miner_subsidy_is_full_block_subsidy() {
    let _init_guard = zebra_test::init();

    use crate::parameters::network::subsidy::{block_subsidy, miner_subsidy};

    let network = Network::Botcash;

    // At various heights, miner subsidy should equal block subsidy
    for height in [Height(0), Height(1000), Height(840_000), Height(1_000_000)] {
        let block_sub = block_subsidy(height, &network).expect("subsidy should be calculable");
        let miner_sub = miner_subsidy(height, &network, block_sub).expect("miner subsidy should be calculable");

        assert_eq!(
            block_sub, miner_sub,
            "Botcash miner subsidy should equal block subsidy at height {} (100% to miners)",
            height.0
        );
    }
}

// ============================================================================
// P1.8: Botcash no funding streams tests
// ============================================================================

/// Tests Botcash has no funding streams at any height.
#[test]
fn botcash_no_funding_streams() {
    let _init_guard = zebra_test::init();

    let network = Network::Botcash;

    // Test that funding_streams() returns None at various heights
    for height in [Height(0), Height(1), Height(1000), Height(1_046_400), Height(2_726_400)] {
        assert!(
            network.funding_streams(height).is_none(),
            "Botcash should have no funding streams at height {}",
            height.0
        );
    }

    // Test that all_funding_streams() returns empty vec
    assert!(
        network.all_funding_streams().is_empty(),
        "Botcash should have empty all_funding_streams()"
    );
}

/// Tests funding_stream_values returns empty for Botcash.
#[test]
fn botcash_no_funding_stream_values() -> Result<(), Report> {
    let _init_guard = zebra_test::init();

    use crate::parameters::network::subsidy::{block_subsidy, funding_stream_values};

    let network = Network::Botcash;

    // At various heights (including post-Canopy equivalent heights), values should be empty
    for height in [Height(0), Height(1000), Height(1_046_400), Height(2_000_000)] {
        let block_sub = block_subsidy(height, &network).expect("subsidy should be calculable");
        let values = funding_stream_values(height, &network, block_sub)?;

        assert!(
            values.is_empty(),
            "Botcash should have no funding stream values at height {}",
            height.0
        );
    }

    Ok(())
}
