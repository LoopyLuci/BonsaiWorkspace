package ai.bonsai.buddy

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.material3.windowsizeclass.calculateWindowSizeClass
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.core.view.WindowCompat
import ai.bonsai.buddy.data.logging.BonsaiLogger
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

    @Inject
    lateinit var logger: BonsaiLogger

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        WindowCompat.setDecorFitsSystemWindows(window, false)

        @OptIn(ExperimentalMaterial3WindowSizeClassApi::class)
        setContent {
            val themeMode = remember { mutableStateOf(secureConfigStore.getThemeMode()) }

            BonsaiBuddyTheme(themeMode = themeMode.value) {
                val windowSizeClass = calculateWindowSizeClass(this)
                val isConfigured = remember {
                    mutableStateOf(
                        secureConfigStore.getConnectionConfig() != null &&
                            !secureConfigStore.getToken().isNullOrBlank()
                    )
                }

                if (isConfigured.value) {
                    BonsaiBuddyApp(
                        windowSizeClass = windowSizeClass,
                        configStore = secureConfigStore,
                        logger = logger
                    )
                } else {
                    OnboardingRoute(onOnboardingComplete = { isConfigured.value = true })
                }
            }
        }
    }
}
