package ai.bonsai.buddy.data.storage

import android.content.Context
import androidx.core.content.edit
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKeys
import ai.bonsai.buddy.data.network.ConnectionConfig
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SecureConfigStore @Inject constructor(
    context: Context
) {
    private val masterKeyAlias = MasterKeys.getOrCreate(MasterKeys.AES256_GCM_SPEC)

    private val prefs = EncryptedSharedPreferences.create(
        PREF_FILE,
        masterKeyAlias,
        context,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun getConnectionConfig(): ConnectionConfig? {
        val host = prefs.getString(KEY_HOST, null) ?: return null
        return ConnectionConfig(
            host = host,
            buddyPort = prefs.getInt(KEY_BUDDY_PORT, 11420),
            workspacePort = prefs.getInt(KEY_WORKSPACE_PORT, 11369),
            useHttps = prefs.getBoolean(KEY_USE_HTTPS, false)
        )
    }

    fun saveConnectionConfig(config: ConnectionConfig) {
        prefs.edit {
            putString(KEY_HOST, config.host)
            putInt(KEY_BUDDY_PORT, config.buddyPort)
            putInt(KEY_WORKSPACE_PORT, config.workspacePort)
            putBoolean(KEY_USE_HTTPS, config.useHttps)
        }
    }

    fun getToken(): String? = prefs.getString(KEY_TOKEN, null)

    fun saveToken(token: String) {
        prefs.edit { putString(KEY_TOKEN, token) }
    }

    fun getThemeMode(): ThemeMode {
        val raw = prefs.getString(KEY_THEME_MODE, ThemeMode.SYSTEM.name) ?: ThemeMode.SYSTEM.name
        return ThemeMode.entries.firstOrNull { it.name == raw } ?: ThemeMode.SYSTEM
    }

    fun setThemeMode(mode: ThemeMode) {
        prefs.edit { putString(KEY_THEME_MODE, mode.name) }
    }

    companion object {
        private const val PREF_FILE = "bonsai_buddy_secure"
        private const val KEY_HOST = "host"
        private const val KEY_BUDDY_PORT = "buddy_port"
        private const val KEY_WORKSPACE_PORT = "workspace_port"
        private const val KEY_USE_HTTPS = "use_https"
        private const val KEY_TOKEN = "desktop_connection_token"
        private const val KEY_THEME_MODE = "theme_mode"
    }
}

enum class ThemeMode {
    SYSTEM,
    LIGHT,
    DARK
}
