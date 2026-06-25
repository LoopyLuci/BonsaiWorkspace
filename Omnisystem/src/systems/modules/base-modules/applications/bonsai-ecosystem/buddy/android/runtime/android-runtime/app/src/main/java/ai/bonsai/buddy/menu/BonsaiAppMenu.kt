package ai.bonsai.buddy.menu

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.drawable.Drawable
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Bonsai app metadata
 */
data class BonsaiAndroidApp(
    val packageName: String,
    val appName: String,
    val description: String,
    val category: AppCategory,
    val icon: String = category.emoji,
    val launchIntent: Intent? = null
)

enum class AppCategory(val emoji: String, val colorHex: Long) {
    AI("🧠", 0xFF6C5CE7),
    INFRASTRUCTURE("📦", 0xFF00B894),
    MEDIA("🎥", 0xFFE17055),
    DEVELOPMENT("🔧", 0xFF0984E3),
    KNOWLEDGE("📚", 0xFFFDCB6E),
    SECURITY("🛡️", 0xFFD63031),
    COMMUNICATION("🤖", 0xFF00CEC9),
    UTILITY("📱", 0xFF636E72)
}

/**
 * Auto-discover all installed Bonsai apps
 */
object BonsaiAppDiscovery {
    private const val BONSAI_PREFIX = "ai.bonsai"

    private val KNOWN_APPS = mapOf(
        "ai.bonsai.buddy" to ("Bonsai Buddy" to "AI companion & assistant"),
        "ai.bonsai.remote" to ("Remote Desktop" to "Control desktops from phone"),
        "ai.bonsai.octopus" to ("Octopus Console" to "Server management on the go"),
        "ai.bonsai.kdb" to ("Knowledge Base" to "Search your knowledge modules"),
        "ai.bonsai.omnibot" to ("OmniBot" to "Chat with your ecosystem"),
        "ai.bonsai.health" to ("Health Tracker" to "Log workouts & nutrition"),
        "ai.bonsai.budget" to ("Budget Tracker" to "Track finances & spending"),
        "ai.bonsai.bug-hunter" to ("Bug Hunter" to "Scan code for issues"),
        "ai.bonsai.media" to ("Media Nexus" to "Stream & record media"),
        "ai.bonsai.devkit" to ("DevKit" to "Build & test tools"),
    )

    private val CATEGORY_MAP = mapOf(
        "ai.bonsai.buddy" to AppCategory.AI,
        "ai.bonsai.octopus" to AppCategory.INFRASTRUCTURE,
        "ai.bonsai.media" to AppCategory.MEDIA,
        "ai.bonsai.devkit" to AppCategory.DEVELOPMENT,
        "ai.bonsai.kdb" to AppCategory.KNOWLEDGE,
        "ai.bonsai.bug-hunter" to AppCategory.SECURITY,
        "ai.bonsai.omnibot" to AppCategory.COMMUNICATION,
        "ai.bonsai.remote" to AppCategory.UTILITY,
        "ai.bonsai.health" to AppCategory.UTILITY,
        "ai.bonsai.budget" to AppCategory.UTILITY,
    )

    fun discover(context: Context): List<BonsaiAndroidApp> {
        val pm = context.packageManager
        val installedApps = mutableListOf<BonsaiAndroidApp>()

        for ((packageName, (appName, description)) in KNOWN_APPS) {
            try {
                pm.getPackageInfo(packageName, 0)  // Check if installed
                val launchIntent = pm.getLaunchIntentForPackage(packageName)
                val category = CATEGORY_MAP[packageName] ?: AppCategory.UTILITY

                installedApps.add(
                    BonsaiAndroidApp(
                        packageName = packageName,
                        appName = appName,
                        description = description,
                        category = category,
                        launchIntent = launchIntent
                    )
                )
            } catch (e: PackageManager.NameNotFoundException) {
                // App not installed, skip
            }
        }

        return installedApps.sortedBy { it.appName }
    }

    fun launch(context: Context, app: BonsaiAndroidApp) {
        app.launchIntent?.let { intent ->
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(intent)
        }
    }
}

/**
 * Composable: Beautiful Bonsai App Menu
 */
@Composable
fun BonsaiAppMenuSheet(
    onDismiss: () -> Unit = {},
    onAppSelected: (BonsaiAndroidApp) -> Unit = { app ->
        BonsaiAppDiscovery.launch(LocalContext.current, app)
    }
) {
    val context = LocalContext.current
    val apps = remember { BonsaiAppDiscovery.discover(context) }
    var searchQuery by remember { mutableStateOf("") }

    val filteredApps = remember(apps, searchQuery) {
        if (searchQuery.isBlank()) {
            apps
        } else {
            apps.filter {
                it.appName.contains(searchQuery, ignoreCase = true) ||
                it.description.contains(searchQuery, ignoreCase = true)
            }
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF1A1A2E))
            .padding(16.dp)
    ) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = 16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "🪴 Bonsai Apps",
                color = Color.White,
                fontSize = 28.sp,
                fontWeight = FontWeight.Bold
            )
            Text(
                text = "${apps.size}",
                color = Color.Gray,
                fontSize = 12.sp
            )
        }

        // Search bar
        TextField(
            value = searchQuery,
            onValueChange = { searchQuery = it },
            placeholder = { Text("Search apps...") },
            modifier = Modifier
                .fillMaxWidth()
                .height(48.dp)
                .padding(bottom = 16.dp),
            shape = RoundedCornerShape(24.dp),
            colors = TextFieldDefaults.colors(
                focusedContainerColor = Color(0xFF16213E),
                unfocusedContainerColor = Color(0xFF16213E),
                focusedTextColor = Color.White,
                unfocusedTextColor = Color.White,
                focusedIndicatorColor = Color.Transparent,
                unfocusedIndicatorColor = Color.Transparent
            ),
            singleLine = true
        )

        if (filteredApps.isEmpty()) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("🌱", fontSize = 64.sp)
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        "No Bonsai apps found",
                        color = Color.Gray,
                        fontSize = 16.sp
                    )
                }
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Fixed(2),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                items(filteredApps) { app ->
                    BonsaiAppCard(
                        app = app,
                        onClick = {
                            onAppSelected(app)
                            onDismiss()
                        }
                    )
                }
            }
        }
    }
}

@Composable
fun BonsaiAppCard(
    app: BonsaiAndroidApp,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = Color(0xFF16213E)
        )
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Category emoji as icon
            Text(
                text = app.category.emoji,
                fontSize = 40.sp,
                modifier = Modifier.padding(bottom = 8.dp)
            )

            // App name
            Text(
                text = app.appName,
                color = Color.White,
                fontSize = 16.sp,
                fontWeight = FontWeight.SemiBold,
                textAlign = TextAlign.Center,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )

            Spacer(modifier = Modifier.height(4.dp))

            // Description
            Text(
                text = app.description,
                color = Color.Gray,
                fontSize = 12.sp,
                textAlign = TextAlign.Center,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )

            Spacer(modifier = Modifier.height(8.dp))

            // Category badge
            Surface(
                shape = RoundedCornerShape(8.dp),
                color = Color(app.category.colorHex).copy(alpha = 0.2f)
            ) {
                Text(
                    text = app.category.name.lowercase(),
                    color = Color(app.category.colorHex),
                    fontSize = 10.sp,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                )
            }
        }
    }
}

/**
 * Preview
 */
@androidx.compose.ui.tooling.preview.Preview
@Composable
fun PreviewBonsaiAppMenu() {
    MaterialTheme {
        BonsaiAppMenuSheet()
    }
}
