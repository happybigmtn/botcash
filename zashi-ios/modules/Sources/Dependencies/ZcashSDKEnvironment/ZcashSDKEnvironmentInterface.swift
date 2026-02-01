//
//  ZcashSDKEnvironmentInterface.swift
//  Botcash
//
//  Created by Lukáš Korba on 13.11.2022.
//  Modified for Botcash network support.
//

import Foundation
import ComposableArchitecture
import ZcashLightClientKit

import Generated
import UserPreferencesStorage

extension DependencyValues {
    public var zcashSDKEnvironment: ZcashSDKEnvironment {
        get { self[ZcashSDKEnvironment.self] }
        set { self[ZcashSDKEnvironment.self] = newValue }
    }
}

extension ZcashSDKEnvironment {
    public enum ZcashSDKConstants {
        // Botcash network endpoints
        static let endpointMainnetAddress = "mainnet.botcash.network"
        static let endpointTestnetAddress = "testnet.botcash.network"
        static let endpointMainnetPort = 9067
        static let endpointTestnetPort = 19067
        static let mnemonicWordsMaxCount = 24
        static let requiredTransactionConfirmations = 10
        public static let streamingCallTimeoutInMillis = Int64(10 * 60 * 60 * 1000) // ten hours
    }
    
    public enum Server: Equatable, Hashable {
        case custom
        case `default`
        case hardcoded(String)
        
        public func desc(for network: NetworkType) -> String? {
            var value: String?
            
            if case .default = self {
                value = L10n.ServerSetup.default
            }
            
            return value
        }
        
        public func value(for network: NetworkType) -> String {
            switch self {
            case .custom:
                return L10n.ServerSetup.custom
            case .default:
                return defaultEndpoint(for: network).server()
            case .hardcoded(let value):
                return value
            }
        }
    }

    public static func servers(for network: NetworkType) -> [Server] {
        var servers = [Server.default]

        if network == .mainnet {
            servers.append(.custom)
            
            let mainnetServers = ZcashSDKEnvironment.endpoints(skipDefault: true).map {
                Server.hardcoded("\($0.host):\($0.port)")
            }
            
            servers.append(contentsOf: mainnetServers)
        } else if network == .testnet {
            servers.append(.custom)
        }
        
        return servers
    }
    
    public static func defaultEndpoint(for network: NetworkType) -> LightWalletEndpoint {
        let defaultHost = network == .mainnet ? ZcashSDKConstants.endpointMainnetAddress : ZcashSDKConstants.endpointTestnetAddress
        let defaultPort = network == .mainnet ? ZcashSDKConstants.endpointMainnetPort : ZcashSDKConstants.endpointTestnetPort

        return LightWalletEndpoint(
            address: defaultHost,
            port: defaultPort,
            secure: true,
            streamingCallTimeoutInMillis: ZcashSDKConstants.streamingCallTimeoutInMillis
        )
    }
    
    public static func endpoints(skipDefault: Bool = false) -> [LightWalletEndpoint] {
        var result: [LightWalletEndpoint] = []

        // Botcash mainnet lightwalletd endpoints
        if !skipDefault {
            result.append(LightWalletEndpoint(address: "mainnet.botcash.network", port: 9067))
        }

        result.append(
            contentsOf: [
                // Geographic distribution for Botcash network
                LightWalletEndpoint(address: "na.botcash.network", port: 9067),
                LightWalletEndpoint(address: "sa.botcash.network", port: 9067),
                LightWalletEndpoint(address: "eu.botcash.network", port: 9067),
                LightWalletEndpoint(address: "ap.botcash.network", port: 9067),
                // Community-run servers
                LightWalletEndpoint(address: "eu.botcash.run", port: 9067),
                LightWalletEndpoint(address: "eu2.botcash.run", port: 9067),
                LightWalletEndpoint(address: "jp.botcash.run", port: 9067),
                LightWalletEndpoint(address: "us.botcash.run", port: 9067)
            ]
        )

        return result
    }
}

@DependencyClient
public struct ZcashSDKEnvironment {
    public var latestCheckpoint: BlockHeight
    public let endpoint: () -> LightWalletEndpoint
    public let exchangeRateIPRateLimit: TimeInterval
    public let exchangeRateStaleLimit: TimeInterval
    public let memoCharLimit: Int
    public let mnemonicWordsMaxCount: Int
    public let network: ZcashNetwork
    public let requiredTransactionConfirmations: Int
    public let sdkVersion: String
    public let serverConfig: () -> UserPreferencesStorage.ServerConfig
    public let servers: [Server]
    public let shieldingThreshold: Zatoshi
    public let tokenName: String
}

extension LightWalletEndpoint {
    public func server() -> String {
        "\(self.host):\(self.port)"
    }
    
    public func serverConfig(isCustom: Bool = false) -> UserPreferencesStorage.ServerConfig {
        UserPreferencesStorage.ServerConfig(host: host, port: port, isCustom: isCustom)
    }
}
