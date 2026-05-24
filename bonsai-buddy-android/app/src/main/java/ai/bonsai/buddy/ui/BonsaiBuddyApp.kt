package ai.bonsai.buddy.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Timeline
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.material3.windowsizeclass.WindowSizeClass
import androidx.compose.material3.windowsizeclass.WindowWidthSizeClass
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import ai.bonsai.buddy.data.logging.BonsaiLogger
import ai.bonsai.buddy.data.storage.SecureConfigStore
import ai.bonsai.buddy.ui.activity.ActivityRoute
import ai.bonsai.buddy.ui.chat.ChatScreen
import ai.bonsai.buddy.ui.chat.ChatViewModel
import ai.bonsai.buddy.ui.models.ModelsRoute
import ai.bonsai.buddy.ui.settings.SettingsScreen
import ai.bonsai.buddy.ui.tools.ToolsRoute

private enum class AppTab(val label: String) {
    Chat("Chat"),
    Tools("Tools"),
    Models("Models"),
    Activity("Activity")
}

@Composable
fun BonsaiBuddyApp(
    windowSizeClass: WindowSizeClass,
    configStore: SecureConfigStore,
    logger: BonsaiLogger
) {
    var selectedTab by remember { mutableStateOf(AppTab.Chat) }
    val width = windowSizeClass.widthSizeClass
    val compact = width == WindowWidthSizeClass.Compact

    NavigationSuiteScaffold(
        navigationSuiteItems = {
            AppTab.entries.forEach { tab ->
                item(
                    selected = selectedTab == tab,
                    onClick = { selectedTab = tab },
                    icon = {
                        Icon(
                            imageVector = tabIcon(tab),
                            contentDescription = tab.label
                        )
                    },
                    label = { Text(tab.label) }
                )
            }
        }
    ) {
        if (compact) {
            CompactContent(
                selectedTab = selectedTab,
                widthSizeClass = width
            )
        } else {
            ExpandedContent(
                selectedTab = selectedTab,
                widthSizeClass = width,
                configStore = configStore,
                logger = logger
            )
        }
    }
}

@Composable
private fun CompactContent(
    selectedTab: AppTab,
    widthSizeClass: WindowWidthSizeClass,
    modifier: Modifier = Modifier
) {
    when (selectedTab) {
        AppTab.Chat -> ChatPane(widthSizeClass = widthSizeClass, modifier = modifier)
        AppTab.Tools -> ToolsRoute(modifier = modifier)
        AppTab.Models -> ModelsRoute(modifier = modifier)
        AppTab.Activity -> ActivityRoute(modifier = modifier)
    }
}

@Composable
private fun ExpandedContent(
    selectedTab: AppTab,
    widthSizeClass: WindowWidthSizeClass,
    configStore: SecureConfigStore,
    logger: BonsaiLogger,
    modifier: Modifier = Modifier
) {
    Row(modifier = modifier.fillMaxSize()) {
        Column(modifier = Modifier.weight(2f).fillMaxHeight()) {
            if (selectedTab == AppTab.Chat) {
                ChatPane(widthSizeClass = widthSizeClass, modifier = Modifier.fillMaxSize())
            } else {
                TabPane(selectedTab = selectedTab, modifier = Modifier.fillMaxSize())
            }
        }

        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxHeight()
                .padding(start = 8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text("Detail Pane", style = MaterialTheme.typography.titleMedium)
            when (selectedTab) {
                AppTab.Chat -> ModelsRoute(modifier = Modifier.fillMaxWidth().weight(1f))
                AppTab.Tools -> ActivityRoute(modifier = Modifier.fillMaxWidth().weight(1f))
                AppTab.Models -> ActivityRoute(modifier = Modifier.fillMaxWidth().weight(1f))
                AppTab.Activity -> ModelsRoute(modifier = Modifier.fillMaxWidth().weight(1f))
            }
            SettingsScreen(
                configStore = configStore,
                logger = logger,
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}

@Composable
private fun TabPane(selectedTab: AppTab, modifier: Modifier = Modifier) {
    when (selectedTab) {
        AppTab.Chat -> Unit
        AppTab.Tools -> ToolsRoute(modifier = modifier)
        AppTab.Models -> ModelsRoute(modifier = modifier)
        AppTab.Activity -> ActivityRoute(modifier = modifier)
    }
}

@Composable
private fun ChatPane(
    widthSizeClass: WindowWidthSizeClass,
    modifier: Modifier = Modifier
) {
    val chatViewModel: ChatViewModel = hiltViewModel()
    val state by chatViewModel.uiState.collectAsState()
    ChatScreen(
        uiState = state,
        widthSizeClass = widthSizeClass,
        onSend = chatViewModel::sendMessage,
        onLoadOlder = chatViewModel::loadOlderMessages,
        modifier = modifier
    )
}

private fun tabIcon(tab: AppTab): ImageVector = when (tab) {
    AppTab.Chat -> Icons.Default.Chat
    AppTab.Tools -> Icons.Default.Build
    AppTab.Models -> Icons.Default.Memory
    AppTab.Activity -> Icons.Default.Timeline
}
