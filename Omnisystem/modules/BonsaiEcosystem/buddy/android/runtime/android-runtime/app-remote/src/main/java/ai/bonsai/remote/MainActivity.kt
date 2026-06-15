package ai.bonsai.remote

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Bundle
import android.os.IBinder
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import ai.bonsai.shared.IBonsaiService

class MainActivity : ComponentActivity() {
    private lateinit var bonsaiService: IBonsaiService
    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            bonsaiService = IBonsaiService.Stub.asInterface(service)
        }
        override fun onServiceDisconnected(name: ComponentName) {}
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        bindService(
            Intent(this, ai.bonsai.shared.service.BonsaiService::class.java),
            connection,
            Context.BIND_AUTO_CREATE
        )

        val peerId = intent.getStringExtra("peerId") ?: "demo-desktop"
        val tokenBase64 = intent.getStringExtra("token") ?: ""

        setContent {
            RemoteDesktopTheme {
                RemoteDesktopScreen(peerId, tokenBase64)
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        try {
            unbindService(connection)
        } catch (e: Exception) {}
    }
}

@Composable
fun RemoteDesktopTheme(content: @Composable () -> Unit) {
    MaterialTheme(content = content)
}

@Composable
fun RemoteDesktopScreen(peerId: String, token: String) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            "🖥️ Bonsai Remote Desktop",
            color = Color.White,
            style = MaterialTheme.typography.headlineLarge,
            modifier = Modifier.padding(16.dp)
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            "Connecting to: $peerId",
            color = Color.Gray,
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier.padding(16.dp)
        )
        Spacer(modifier = Modifier.height(24.dp))
        CircularProgressIndicator(color = Color.White)
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            "Establishing secure connection...",
            color = Color.White,
            style = MaterialTheme.typography.bodyMedium
        )

        // TODO: Initialize remote desktop streaming
        // TODO: Show video decoder output
        // TODO: Add touch input overlay
    }
}
