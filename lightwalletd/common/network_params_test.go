// Copyright (c) 2019-2020 The Zcash developers
// Copyright (c) 2026 The Botcash developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or https://www.opensource.org/licenses/mit-license.php .

package common

import (
	"regexp"
	"testing"
)

func TestBotcashNetworkParams(t *testing.T) {
	params := GetNetworkParams("botcash")
	if params == nil {
		t.Fatal("Expected botcash network params to exist")
	}
	if params.Name != "botcash" {
		t.Errorf("Expected name 'botcash', got '%s'", params.Name)
	}
	if params.RPCDefaultPort != "8532" {
		t.Errorf("Expected RPC port '8532', got '%s'", params.RPCDefaultPort)
	}
	if params.TaddrPrefixRegex != "^B1" {
		t.Errorf("Expected taddr prefix '^B1', got '%s'", params.TaddrPrefixRegex)
	}
}

func TestBotcashTestNetworkParams(t *testing.T) {
	params := GetNetworkParams("botcash-test")
	if params == nil {
		t.Fatal("Expected botcash-test network params to exist")
	}
	if params.Name != "botcash-test" {
		t.Errorf("Expected name 'botcash-test', got '%s'", params.Name)
	}
	if params.RPCDefaultPort != "18532" {
		t.Errorf("Expected RPC port '18532', got '%s'", params.RPCDefaultPort)
	}
	if params.TaddrPrefixRegex != "^B1" {
		t.Errorf("Expected taddr prefix '^B1', got '%s'", params.TaddrPrefixRegex)
	}
}

func TestZcashNetworkParams(t *testing.T) {
	// Test mainnet
	params := GetNetworkParams("main")
	if params == nil {
		t.Fatal("Expected main network params to exist")
	}
	if params.RPCDefaultPort != "8232" {
		t.Errorf("Expected RPC port '8232', got '%s'", params.RPCDefaultPort)
	}
	if params.TaddrPrefixRegex != "^t1" {
		t.Errorf("Expected taddr prefix '^t1', got '%s'", params.TaddrPrefixRegex)
	}

	// Test testnet
	params = GetNetworkParams("test")
	if params == nil {
		t.Fatal("Expected test network params to exist")
	}
	if params.RPCDefaultPort != "18232" {
		t.Errorf("Expected RPC port '18232', got '%s'", params.RPCDefaultPort)
	}
}

func TestGetDefaultRPCPort(t *testing.T) {
	tests := []struct {
		chainName    string
		expectedPort string
	}{
		{"main", "8232"},
		{"test", "18232"},
		{"regtest", "18232"},
		{"botcash", "8532"},
		{"botcash-test", "18532"},
		{"unknown", "8232"}, // Falls back to Zcash mainnet
	}

	for _, tt := range tests {
		t.Run(tt.chainName, func(t *testing.T) {
			port := GetDefaultRPCPort(tt.chainName)
			if port != tt.expectedPort {
				t.Errorf("GetDefaultRPCPort(%s) = %s, want %s", tt.chainName, port, tt.expectedPort)
			}
		})
	}
}

func TestGetTaddrPrefixRegex(t *testing.T) {
	tests := []struct {
		chainName      string
		expectedPrefix string
	}{
		{"main", "^t1"},
		{"test", "^tm"},
		{"botcash", "^B1"},
		{"botcash-test", "^B1"},
		{"unknown", "^t1"}, // Falls back to Zcash mainnet
	}

	for _, tt := range tests {
		t.Run(tt.chainName, func(t *testing.T) {
			prefix := GetTaddrPrefixRegex(tt.chainName)
			if prefix != tt.expectedPrefix {
				t.Errorf("GetTaddrPrefixRegex(%s) = %s, want %s", tt.chainName, prefix, tt.expectedPrefix)
			}
		})
	}
}

func TestIsBotcashNetwork(t *testing.T) {
	tests := []struct {
		chainName string
		expected  bool
	}{
		{"botcash", true},
		{"botcash-test", true},
		{"main", false},
		{"test", false},
		{"regtest", false},
		{"unknown", false},
	}

	for _, tt := range tests {
		t.Run(tt.chainName, func(t *testing.T) {
			result := IsBotcashNetwork(tt.chainName)
			if result != tt.expected {
				t.Errorf("IsBotcashNetwork(%s) = %v, want %v", tt.chainName, result, tt.expected)
			}
		})
	}
}

func TestIsZcashNetwork(t *testing.T) {
	tests := []struct {
		chainName string
		expected  bool
	}{
		{"main", true},
		{"test", true},
		{"regtest", true},
		{"botcash", false},
		{"botcash-test", false},
		{"unknown", false},
	}

	for _, tt := range tests {
		t.Run(tt.chainName, func(t *testing.T) {
			result := IsZcashNetwork(tt.chainName)
			if result != tt.expected {
				t.Errorf("IsZcashNetwork(%s) = %v, want %v", tt.chainName, result, tt.expected)
			}
		})
	}
}

func TestBotcashAddressPrefixRegex(t *testing.T) {
	// Test that the B1 prefix regex correctly matches Botcash addresses
	regex := regexp.MustCompile(GetTaddrPrefixRegex("botcash") + "[a-zA-Z0-9]{33}$")

	validAddresses := []string{
		"B1abcdefghijklmnopqrstuvwxyz123456", // 35 chars total (B1 + 33)
		"B1ABCDEFGHIJKLMNOPQRSTUVWXYZ123456", // uppercase
	}

	invalidAddresses := []string{
		"t1abcdefghijklmnopqrstuvwxyz123456", // Zcash prefix
		"b1abcdefghijklmnopqrstuvwxyz123456", // lowercase b1
		"B2abcdefghijklmnopqrstuvwxyz123456", // B2 prefix
		"B1abc",                               // too short
	}

	for _, addr := range validAddresses {
		if !regex.MatchString(addr) {
			t.Errorf("Expected address %s to match Botcash pattern", addr)
		}
	}

	for _, addr := range invalidAddresses {
		if regex.MatchString(addr) {
			t.Errorf("Expected address %s to NOT match Botcash pattern", addr)
		}
	}
}

func TestZcashAddressPrefixRegex(t *testing.T) {
	// Test that the t1 prefix regex correctly matches Zcash addresses
	regex := regexp.MustCompile(GetTaddrPrefixRegex("main") + "[a-zA-Z0-9]{33}$")

	validAddresses := []string{
		"t1abcdefghijklmnopqrstuvwxyz123456", // 35 chars total (t1 + 33)
	}

	invalidAddresses := []string{
		"B1abcdefghijklmnopqrstuvwxyz123456", // Botcash prefix
		"t1abc",                               // too short
	}

	for _, addr := range validAddresses {
		if !regex.MatchString(addr) {
			t.Errorf("Expected address %s to match Zcash pattern", addr)
		}
	}

	for _, addr := range invalidAddresses {
		if regex.MatchString(addr) {
			t.Errorf("Expected address %s to NOT match Zcash pattern", addr)
		}
	}
}
