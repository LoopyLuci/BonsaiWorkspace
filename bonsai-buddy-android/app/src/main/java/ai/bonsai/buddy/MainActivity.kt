package ai.bonsai.buddy

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.windowsizeclass.calculateWindowSizeClass
import androidx.core.view.WindowCompat
import ai.bonsai.buddy.ui.BonsaiBuddyApp
import ai.bonsai.buddy.ui.theme.BonsaiBuddyTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        WindowCompat.setDecorFitsSystemWindows(window, false)

        setContent {
            BonsaiBuddyTheme {
                val windowSizeClass = calculateWindowSizeClass(this)
                BonsaiBuddyApp(windowSizeClass = windowSizeClass)
            }
        }
    }
}
