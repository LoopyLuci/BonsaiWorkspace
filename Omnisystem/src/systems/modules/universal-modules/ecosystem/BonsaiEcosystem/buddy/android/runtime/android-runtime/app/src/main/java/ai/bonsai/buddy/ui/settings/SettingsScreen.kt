package ai.bonsai.buddy.ui.settings

import android.content.Intent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AssistChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import ai.bonsai.buddy.data.logging.BonsaiLogger
import ai.bonsai.buddy.data.storage.SecureConfigStore
import ai.bonsai.buddy.data.storage.ThemeMode

@Composable
fun SettingsScreen(
    configStore: SecureConfigStore,
    logger: BonsaiLogger,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current

    Column(
        modifier = modifier.fillMaxWidth().padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        Text("Settings", style = MaterialTheme.typography.titleLarge)
        Text("Theme", style = MaterialTheme.typography.titleMedium)
        ThemeMode.entries.forEach { mode ->
            AssistChip(
                onClick = { configStore.setThemeMode(mode) },
                label = { Text(mode.name) }
            )
        }

        AssistChip(
            onClick = {
                val file = logger.getLogFile()
                val uri = FileProvider.getUriForFile(context, logger.shareLogAuthority(), file)
                val intent = Intent(Intent.ACTION_SEND).apply {
                    type = "text/plain"
                    putExtra(Intent.EXTRA_STREAM, uri)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                context.startActivity(Intent.createChooser(intent, "Share Bonsai Logs"))
            },
            label = { Text("Share Logs") }
        )
    }
}
