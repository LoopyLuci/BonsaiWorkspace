package ai.bonsai.buddy.ui.remote_desktop

import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.ui.unit.sp

@Composable
fun OnScreenKeyboard(
    onKeyPress: (String) -> Unit = {},
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier) {
        Row {
            Button(onClick = { onKeyPress("A") }) {
                Text("A", fontSize = 12.sp)
            }
            Button(onClick = { onKeyPress("B") }) {
                Text("B", fontSize = 12.sp)
            }
        }
    }
}
