package ai.omnisystem.buddy

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.windowsizeclass.calculateWindowSizeClass
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.core.view.WindowCompat
import ai.omnisystem.buddy.data.logging.OmnisystemLogger
import ai.omnisystem.buddy.data.storage.SecureConfigStore
import ai.omnisystem.buddy.ui.OmnisystemBuddyApp
import ai.omnisystem.buddy.ui.onboarding.OnboardingRoute
import ai.omnisystem.buddy.ui.theme.OmnisystemBuddyTheme
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    @Inject
    lateinit var secureConfigStore: SecureConfigStore

    @Inject
    lateinit var logger: OmnisystemLogger

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        WindowCompat.setDecorFitsSystemWindows(window, false)

        setContent {
            val themeMode = remember { mutableStateOf(secureConfigStore.getThemeMode()) }

            OmnisystemBuddyTheme(themeMode = themeMode.value) {
                val windowSizeClass = calculateWindowSizeClass(this)
                val isConfigured = remember {
                    mutableStateOf(
                        secureConfigStore.getConnectionConfig() != null &&
                            !secureConfigStore.getToken().isNullOrBlank()
                    )
                }

                if (isConfigured.value) {
                    OmnisystemBuddyApp(
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
