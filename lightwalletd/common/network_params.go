// Copyright (c) 2019-2020 The Zcash developers
// Copyright (c) 2026 The Botcash developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or https://www.opensource.org/licenses/mit-license.php .

package common

// NetworkParams defines network-specific parameters for different blockchain networks.
// This includes RPC ports, address prefixes, and other network-specific configuration.
type NetworkParams struct {
	Name                    string // Network name as returned by getblockchaininfo RPC
	RPCDefaultPort          string // Default RPC port for this network
	TaddrPrefixRegex        string // Regex pattern for transparent address prefix validation
	SaplingActivationHeight uint64 // Block height at which Sapling activated
}

// Networks maps chain names to their network parameters.
// The chain name is obtained from the "chain" field in getblockchaininfo RPC response.
var Networks = map[string]*NetworkParams{
	// Zcash networks
	"main": {
		Name:                    "main",
		RPCDefaultPort:          "8232",
		TaddrPrefixRegex:        "^t1",
		SaplingActivationHeight: 419200,
	},
	"test": {
		Name:                    "test",
		RPCDefaultPort:          "18232",
		TaddrPrefixRegex:        "^tm",
		SaplingActivationHeight: 280000,
	},
	"regtest": {
		Name:                    "regtest",
		RPCDefaultPort:          "18232",
		TaddrPrefixRegex:        "^tm",
		SaplingActivationHeight: 1,
	},

	// Botcash networks
	"botcash": {
		Name:                    "botcash",
		RPCDefaultPort:          "8532",
		TaddrPrefixRegex:        "^B1",
		SaplingActivationHeight: 1, // Sapling active from genesis on Botcash
	},
	"botcash-test": {
		Name:                    "botcash-test",
		RPCDefaultPort:          "18532",
		TaddrPrefixRegex:        "^B1",
		SaplingActivationHeight: 1, // Sapling active from genesis on Botcash testnet
	},
}

// GetNetworkParams returns the network parameters for a given chain name.
// Returns nil if the chain name is not recognized.
func GetNetworkParams(chainName string) *NetworkParams {
	if params, ok := Networks[chainName]; ok {
		return params
	}
	return nil
}

// GetDefaultRPCPort returns the default RPC port for a given chain name.
// Falls back to Zcash mainnet port (8232) if the chain is not recognized.
func GetDefaultRPCPort(chainName string) string {
	if params := GetNetworkParams(chainName); params != nil {
		return params.RPCDefaultPort
	}
	return "8232" // Default fallback to Zcash mainnet
}

// GetTaddrPrefixRegex returns the transparent address prefix regex for a chain.
// Falls back to Zcash mainnet prefix (^t1) if the chain is not recognized.
func GetTaddrPrefixRegex(chainName string) string {
	if params := GetNetworkParams(chainName); params != nil {
		return params.TaddrPrefixRegex
	}
	return "^t1" // Default fallback to Zcash mainnet
}

// IsBotcashNetwork returns true if the chain name indicates a Botcash network.
func IsBotcashNetwork(chainName string) bool {
	return chainName == "botcash" || chainName == "botcash-test"
}

// IsZcashNetwork returns true if the chain name indicates a Zcash network.
func IsZcashNetwork(chainName string) bool {
	return chainName == "main" || chainName == "test" || chainName == "regtest"
}
