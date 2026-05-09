package ai.bonsai.buddy.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Timeline
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.windowsizeclass.WindowSizeClass
import androidx.compose.material3.windowsizeclass.WindowWidthSizeClass
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import ai.bonsai.buddy.ui.chat.ChatScreen
import ai.bonsai.buddy.ui.chat.ChatViewModel

private enum class AppTab(val label: String) {
    Chat("Chat"),
    Tools("Tools"),
    Models("Models"),
    Activity("Activity")
}

@Composable
fun BonsaiBuddyApp(windowSizeClass: WindowSizeClass) {
    var selectedTab by remember { mutableStateOf(AppTab.Chat) }
    val isCompact = windowSizeClass.widthSizeClass == WindowWidthSizeClass.Compact

    if (isCompact) {
        Scaffold(
            bottomBar = {
                NavigationBar {
                    AppTab.entries.forEach { tab ->
                        NavigationBarItem(
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
            }
        ) { padding ->
            AppContent(
                selectedTab = selectedTab,
                widthSizeClass = windowSizeClass.widthSizeClass,
                modifier = Modifier.padding(padding)
            )
        }
    } else {
        Row(modifier = Modifier.fillMaxSize()) {
            NavigationRail {
                AppTab.entries.forEach { tab ->
                    NavigationRailItem(
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
            AppContent(
                selectedTab = selectedTab,
                widthSizeClass = windowSizeClass.widthSizeClass,
                modifier = Modifier.weight(1f)
            )
        }
    }
}

@Composable
private fun AppContent(
    selectedTab: AppTab,
    widthSizeClass: WindowWidthSizeClass,
    modifier: Modifier = Modifier
) {
    val chatViewModel: ChatViewModel = hiltViewModel()
    val chatState by chatViewModel.uiState.collectAsState()
    when (selectedTab) {
        AppTab.Chat -> ChatScreen(
            uiState = chatState,
            widthSizeClass = widthSizeClass,
            onSend = chatViewModel::sendMessage,
            onLoadOlder = chatViewModel::loadOlderMessages,
            modifier = modifier
        )

        AppTab.Tools,
        AppTab.Models,
        AppTab.Activity -> PlaceholderPane(
            title = selectedTab.label,
            modifier = modifier
        )
    }
}

@Composable
private fun PlaceholderPane(title: String, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(text = title, style = MaterialTheme.typography.headlineSmall)
            Text(
                text = "Implemented in next sprint.",
                style = MaterialTheme.typography.bodyMedium
            )
        }
    }
}

private fun tabIcon(tab: AppTab): ImageVector = when (tab) {
    AppTab.Chat -> Icons.Default.Chat
    AppTab.Tools -> Icons.Default.Build
    AppTab.Models -> Icons.Default.Memory
    AppTab.Activity -> Icons.Default.Timeline
}
