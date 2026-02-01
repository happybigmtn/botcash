//
//  NetworkTests.swift
//  secantTests
//
//  Created for Botcash network verification.
//

import XCTest
import ComposableArchitecture
import ZcashLightClientKit
import ZcashSDKEnvironment
@testable import secant_testnet

@MainActor
class NetworkTests: XCTestCase {

    // MARK: - P3.1: Endpoint Constants Tests

    func testBotcashMainnetEndpointAddress() throws {
        // Verify mainnet endpoint is configured for Botcash network
        let endpoint = ZcashSDKEnvironment.defaultEndpoint(for: .mainnet)
        XCTAssertEqual(endpoint.host, "mainnet.botcash.network")
        XCTAssertEqual(endpoint.port, 9067)
    }

    func testBotcashTestnetEndpointAddress() throws {
        // Verify testnet endpoint is configured for Botcash network
        let endpoint = ZcashSDKEnvironment.defaultEndpoint(for: .testnet)
        XCTAssertEqual(endpoint.host, "testnet.botcash.network")
        XCTAssertEqual(endpoint.port, 19067)
    }

    func testBotcashEndpointsCount() throws {
        // Verify we have the expected number of endpoints (9 total: 1 default + 8 additional)
        let endpoints = ZcashSDKEnvironment.endpoints(skipDefault: false)
        XCTAssertEqual(endpoints.count, 9)
    }

    func testBotcashEndpointsIncludeDefault() throws {
        // Verify default endpoint is included when not skipped
        let endpointsWithDefault = ZcashSDKEnvironment.endpoints(skipDefault: false)
        let hasDefault = endpointsWithDefault.contains { $0.host == "mainnet.botcash.network" && $0.port == 9067 }
        XCTAssertTrue(hasDefault, "Default endpoint should be included")
    }

    func testBotcashEndpointsSkipDefault() throws {
        // Verify default endpoint is NOT included when skipped
        let endpointsWithoutDefault = ZcashSDKEnvironment.endpoints(skipDefault: true)
        let hasDefault = endpointsWithoutDefault.contains { $0.host == "mainnet.botcash.network" && $0.port == 9067 }
        XCTAssertFalse(hasDefault, "Default endpoint should not be included when skipped")
        XCTAssertEqual(endpointsWithoutDefault.count, 8)
    }

    func testBotcashEndpointsAllUseCorrectPort() throws {
        // Verify all endpoints use the Botcash mainnet gRPC port
        let endpoints = ZcashSDKEnvironment.endpoints(skipDefault: false)
        for endpoint in endpoints {
            XCTAssertEqual(endpoint.port, 9067, "Endpoint \(endpoint.host) should use port 9067")
        }
    }

    func testBotcashEndpointsGeographicDistribution() throws {
        // Verify we have geographically distributed endpoints
        let endpoints = ZcashSDKEnvironment.endpoints(skipDefault: false)
        let hosts = endpoints.map { $0.host }

        // Check for regional endpoints
        XCTAssertTrue(hosts.contains("na.botcash.network"), "Should have North America endpoint")
        XCTAssertTrue(hosts.contains("sa.botcash.network"), "Should have South America endpoint")
        XCTAssertTrue(hosts.contains("eu.botcash.network"), "Should have Europe endpoint")
        XCTAssertTrue(hosts.contains("ap.botcash.network"), "Should have Asia-Pacific endpoint")
    }

    func testBotcashEndpointsCommunityServers() throws {
        // Verify community-run servers are included
        let endpoints = ZcashSDKEnvironment.endpoints(skipDefault: false)
        let hosts = endpoints.map { $0.host }

        // Check for community-run servers
        XCTAssertTrue(hosts.contains("eu.botcash.run"), "Should have EU community server")
        XCTAssertTrue(hosts.contains("us.botcash.run"), "Should have US community server")
        XCTAssertTrue(hosts.contains("jp.botcash.run"), "Should have JP community server")
    }

    func testBotcashMainnetTokenName() throws {
        // Verify mainnet uses BCASH as token name
        let environment = ZcashSDKEnvironment.live(network: ZcashNetworkBuilder.network(for: .mainnet))
        XCTAssertEqual(environment.tokenName, "BCASH")
    }

    func testBotcashTestnetTokenName() throws {
        // Verify testnet uses tBCASH as token name
        let environment = ZcashSDKEnvironment.live(network: ZcashNetworkBuilder.network(for: .testnet))
        XCTAssertEqual(environment.tokenName, "tBCASH")
    }

    func testBotcashMainnetServers() throws {
        // Verify servers list for mainnet includes default, custom, and hardcoded servers
        let servers = ZcashSDKEnvironment.servers(for: .mainnet)

        // Should have: default + custom + 8 hardcoded = 10 servers
        XCTAssertGreaterThanOrEqual(servers.count, 10)

        // First should be default
        if case .default = servers[0] {
            XCTAssertTrue(true)
        } else {
            XCTFail("First server should be default")
        }

        // Second should be custom
        if case .custom = servers[1] {
            XCTAssertTrue(true)
        } else {
            XCTFail("Second server should be custom")
        }
    }

    func testBotcashTestnetServers() throws {
        // Verify servers list for testnet includes default and custom only
        let servers = ZcashSDKEnvironment.servers(for: .testnet)

        // Should have: default + custom = 2 servers
        XCTAssertEqual(servers.count, 2)
    }

    func testBotcashDefaultServerValue() throws {
        // Verify default server returns correct value for mainnet
        let server = ZcashSDKEnvironment.Server.default
        let value = server.value(for: .mainnet)
        XCTAssertEqual(value, "mainnet.botcash.network:9067")
    }

    func testBotcashEndpointSecure() throws {
        // Verify default endpoint uses secure connection
        let endpoint = ZcashSDKEnvironment.defaultEndpoint(for: .mainnet)
        XCTAssertTrue(endpoint.secure, "Endpoint should use secure connection")
    }
}
