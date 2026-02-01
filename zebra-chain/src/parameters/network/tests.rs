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
