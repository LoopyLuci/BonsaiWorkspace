package ai.bonsai.buddy

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.material3.windowsizeclass.calculateWindowSizeClass
import androidx.core.view.WindowCompat
import ai.bonsai.buddy.data.storage.SecureConfigStore
import ai.bonsai.buddy.ui.BonsaiBuddyApp
import ai.bonsai.buddy.ui.onboarding.OnboardingRoute
import ai.bonsai.buddy.ui.theme.BonsaiBuddyTheme
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    @Inject
    lateinit var secureConfigStore: SecureConfigStore

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        WindowCompat.setDecorFitsSystemWindows(window, false)

        setContent {
            BonsaiBuddyTheme {
                val windowSizeClass = calculateWindowSizeClass(this)
                val isConfigured = remember {
                    mutableStateOf(
                        secureConfigStore.getConnectionConfig() != null &&
                            !secureConfigStore.getToken().isNullOrBlank()
                    )
                }

                if (isConfigured.value) {
                    BonsaiBuddyApp(windowSizeClass = windowSizeClass)
                } else {
                    OnboardingRoute(onOnboardingComplete = { isConfigured.value = true })
                }
            }
        }
    }
}
