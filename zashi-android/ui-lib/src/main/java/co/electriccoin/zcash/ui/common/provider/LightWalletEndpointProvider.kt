package co.electriccoin.zcash.ui.common.provider

import android.app.Application
import cash.z.ecc.android.sdk.model.ZcashNetwork
import cash.z.ecc.sdk.type.fromResources
import co.electriccoin.lightwallet.client.model.LightWalletEndpoint

// Botcash lightwalletd endpoints
// Port 9067 is the standard Botcash lightwalletd gRPC port
class LightWalletEndpointProvider(
    private val application: Application
) {
    fun getEndpoints(): List<LightWalletEndpoint> =
        if (ZcashNetwork.fromResources(application) == ZcashNetwork.Mainnet) {
            listOf(
                // Botcash mainnet endpoints
                LightWalletEndpoint(host = "mainnet.botcash.network", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "na.botcash.network", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "sa.botcash.network", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "eu.botcash.network", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "ap.botcash.network", port = 9067, isSecure = true),
                // Community-run servers
                LightWalletEndpoint(host = "eu.botcash.run", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "eu2.botcash.run", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "jp.botcash.run", port = 9067, isSecure = true),
                LightWalletEndpoint(host = "us.botcash.run", port = 9067, isSecure = true),
            )
        } else {
            listOf(
                // Botcash testnet endpoint
                LightWalletEndpoint(host = "testnet.botcash.network", port = 19067, isSecure = true)
            )
        }

    fun getDefaultEndpoint() = getEndpoints().first()
}
