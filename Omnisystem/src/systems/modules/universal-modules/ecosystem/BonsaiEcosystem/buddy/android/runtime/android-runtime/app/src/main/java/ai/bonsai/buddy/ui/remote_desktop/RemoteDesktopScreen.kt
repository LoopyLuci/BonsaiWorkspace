package ai.bonsai.buddy.ui.remote_desktop

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color

@Composable
fun RemoteDesktopScreen(
    peerId: String = "default",
    tokenBase64: String = "",
    onNavigateBack: () -> Unit = {}
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        Text("Remote Desktop", color = Color.White)
        Button(onClick = onNavigateBack) {
            Text("Back")
        }
    }
}
