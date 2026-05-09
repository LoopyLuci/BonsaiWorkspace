package ai.bonsai.buddy.data.discovery

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class NsdDiscoveryManager @Inject constructor(
    context: Context
) {
    private val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager

    fun discoverServices(serviceType: String = "_http._tcp."): Flow<List<NsdServiceInfo>> = callbackFlow {
        val discovered = mutableListOf<NsdServiceInfo>()

        val listener = object : NsdManager.DiscoveryListener {
            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                close(IllegalStateException("NSD start failed: $errorCode"))
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                close(IllegalStateException("NSD stop failed: $errorCode"))
            }

            override fun onDiscoveryStarted(serviceType: String) = Unit
            override fun onDiscoveryStopped(serviceType: String) = Unit

            override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                discovered.add(serviceInfo)
                trySend(discovered.toList())
            }

            override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                discovered.removeAll { it.serviceName == serviceInfo.serviceName }
                trySend(discovered.toList())
            }
        }

        nsdManager.discoverServices(serviceType, NsdManager.PROTOCOL_DNS_SD, listener)

        awaitClose {
            runCatching { nsdManager.stopServiceDiscovery(listener) }
        }
    }
}
