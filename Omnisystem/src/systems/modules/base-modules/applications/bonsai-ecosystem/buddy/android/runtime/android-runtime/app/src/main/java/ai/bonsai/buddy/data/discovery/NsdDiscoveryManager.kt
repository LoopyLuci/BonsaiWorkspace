package ai.bonsai.buddy.data.discovery

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.flowOf
import javax.inject.Inject
import javax.inject.Singleton

data class DiscoveredWorkspace(
    val name: String,
    val host: String,
    val port: Int,
    val source: DiscoverySource
)

enum class DiscoverySource {
    NSD,
    MOCK,
    MANUAL
}

@Singleton
class NsdDiscoveryManager @Inject constructor(
    context: Context
) {
    private val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager

    fun discoverServices(serviceType: String = "_bonsai._tcp."): Flow<List<NsdServiceInfo>> = callbackFlow {
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
                val resolver = object : NsdManager.ResolveListener {
                    override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                        discovered.add(serviceInfo)
                        trySend(discovered.toList())
                    }

                    override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                        discovered.removeAll { it.serviceName == serviceInfo.serviceName }
                        discovered.add(serviceInfo)
                        trySend(discovered.toList())
                    }
                }

                runCatching { nsdManager.resolveService(serviceInfo, resolver) }
                    .onFailure {
                        discovered.add(serviceInfo)
                        trySend(discovered.toList())
                    }
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

    fun discoverWorkspaces(): Flow<List<DiscoveredWorkspace>> {
        val mock = flowOf(
            listOf(
                DiscoveredWorkspace(
                    name = "Local Bonsai (Mock)",
                    host = "192.168.1.20",
                    port = 11420,
                    source = DiscoverySource.MOCK
                )
            )
        )

        return discoverServices()
            .catch { emit(emptyList()) }
            .combine(mock) { services, mocks ->
                val nsdItems = services.mapNotNull { service ->
                    val host = service.host?.hostAddress ?: return@mapNotNull null
                    DiscoveredWorkspace(
                        name = service.serviceName,
                        host = host,
                        port = service.port,
                        source = DiscoverySource.NSD
                    )
                }
                (nsdItems + mocks).distinctBy { "${it.host}:${it.port}" }
            }
    }
}
