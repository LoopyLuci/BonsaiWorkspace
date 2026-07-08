package ai.omnisystem.buddy.data.network

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.R
import android.os.IBinder
import androidx.core.app.NotificationCompat
import ai.omnisystem.buddy.MainActivity
import ai.omnisystem.buddy.data.logging.OmnisystemLogger
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.launch

@AndroidEntryPoint
class OmnisystemWebSocketService : Service() {
    @Inject
    lateinit var apiClient: OmnisystemApiClient

    @Inject
    lateinit var logger: OmnisystemLogger

    private val scope = CoroutineScope(Dispatchers.IO + Job())
    private var streamJob: Job? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, baseNotification("Connected to Omnisystem"))
        connectStream()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_DISCONNECT) {
            stopSelf()
            return START_NOT_STICKY
        }
        return START_STICKY
    }

    override fun onDestroy() {
        streamJob?.cancel()
        scope.cancel()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun connectStream() {
        streamJob?.cancel()
        streamJob = scope.launch {
            logger.i(TAG, "Starting websocket/event stream")
            apiClient.eventStream()
                .catch { err ->
                    logger.e(TAG, "Event stream disconnected", err)
                    postEventNotification("Disconnected", "Attempting reconnect")
                    connectStream()
                }
                .collect { event ->
                    logger.d(TAG, "Event: $event")
                    if (event.contains("completed", ignoreCase = true) ||
                        event.contains("loaded", ignoreCase = true)
                    ) {
                        postEventNotification("Omnisystem Event", event.take(120))
                    }
                }
        }
    }

    private fun baseNotification(content: String): Notification {
        val openIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val disconnectIntent = PendingIntent.getService(
            this,
            1,
            Intent(this, OmnisystemWebSocketService::class.java).setAction(ACTION_DISCONNECT),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.stat_notify_sync)
            .setContentTitle("Omnisystem Buddy")
            .setContentText(content)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .setContentIntent(openIntent)
            .addAction(0, "Open Chat", openIntent)
            .addAction(0, "Disconnect", disconnectIntent)
            .build()
    }

    private fun postEventNotification(title: String, body: String) {
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.notify(
            (System.currentTimeMillis() % Int.MAX_VALUE).toInt(),
            NotificationCompat.Builder(this, CHANNEL_ID)
                .setSmallIcon(R.drawable.stat_notify_sync)
                .setContentTitle(title)
                .setContentText(body)
                .setPriority(NotificationCompat.PRIORITY_DEFAULT)
                .build()
        )
    }

    private fun createNotificationChannel() {
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Omnisystem Connectivity",
            NotificationManager.IMPORTANCE_DEFAULT
        )
        manager.createNotificationChannel(channel)
    }

    companion object {
        private const val TAG = "OmnisystemWebSocketService"
        private const val CHANNEL_ID = "omnisystem-connectivity"
        private const val NOTIFICATION_ID = 42001
        private const val ACTION_DISCONNECT = "ai.omnisystem.buddy.DISCONNECT"
    }
}
