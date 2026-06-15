package ai.bonsai.shared.training

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.time.Instant

/**
 * Exports training data snapshots (TDL format) for model training.
 * Handles data collection, formatting, and compression.
 */
class TdlExporter(private val context: Context) {
    private val tdlDir = File(context.getExternalFilesDir(null), "tdl")
    private val snapshotDir = File(context.getExternalFilesDir(null), "snapshots")

    init {
        tdlDir.mkdirs()
        snapshotDir.mkdirs()
    }

    /**
     * Export current training data as a TDL snapshot.
     *
     * @param name Snapshot name
     * @param includeChat Include chat history
     * @param includeActivity Include activity logs
     * @param onProgress Progress callback (current, total)
     * @return Path to exported snapshot or null
     */
    suspend fun exportSnapshot(
        name: String,
        includeChat: Boolean = true,
        includeActivity: Boolean = true,
        onProgress: (Long, Long) -> Unit = { _, _ -> }
    ): String? = withContext(Dispatchers.Default) {
        return@withContext try {
            val timestamp = Instant.now().epochSecond
            val snapshotFile = File(snapshotDir, "$name-$timestamp.tdl")

            // Collect training data
            val data = collectTrainingData(
                includeChat = includeChat,
                includeActivity = includeActivity,
                onProgress = onProgress
            ) ?: return@withContext null

            // Export to TDL format
            val success = nativeExportTdl(
                data,
                snapshotFile.absolutePath,
                name
            )

            if (success) {
                Log.d(TAG, "Snapshot exported: ${snapshotFile.absolutePath}")
                snapshotFile.absolutePath
            } else {
                Log.e(TAG, "Export failed")
                null
            }
        } catch (e: Exception) {
            Log.e(TAG, "Export error", e)
            null
        }
    }

    /**
     * Sync training data to remote storage.
     *
     * @param snapshotPath Path to snapshot to sync
     * @param remoteUrl Remote storage URL
     * @param onProgress Progress callback
     * @return Success status
     */
    suspend fun syncSnapshot(
        snapshotPath: String,
        remoteUrl: String,
        onProgress: (Long, Long) -> Unit = { _, _ -> }
    ): Boolean = withContext(Dispatchers.Default) {
        return@withContext try {
            val file = File(snapshotPath)
            if (!file.exists()) {
                Log.e(TAG, "Snapshot not found: $snapshotPath")
                return@withContext false
            }

            // Upload via native transfer
            nativeSyncSnapshot(
                snapshotPath,
                remoteUrl,
                { current, total -> onProgress(current, total) }
            )
        } catch (e: Exception) {
            Log.e(TAG, "Sync error", e)
            false
        }
    }

    /**
     * List all available snapshots.
     */
    suspend fun listSnapshots(): List<SnapshotInfo> = withContext(Dispatchers.Default) {
        return@withContext try {
            snapshotDir.listFiles()?.mapNotNull { file ->
                if (file.extension == "tdl") {
                    SnapshotInfo(
                        name = file.nameWithoutExtension,
                        path = file.absolutePath,
                        size = file.length(),
                        modified = file.lastModified()
                    )
                } else {
                    null
                }
            } ?: emptyList()
        } catch (e: Exception) {
            Log.e(TAG, "Error listing snapshots", e)
            emptyList()
        }
    }

    /**
     * Delete a snapshot.
     */
    suspend fun deleteSnapshot(snapshotPath: String): Boolean = withContext(Dispatchers.Default) {
        return@withContext try {
            File(snapshotPath).delete()
        } catch (e: Exception) {
            Log.e(TAG, "Error deleting snapshot", e)
            false
        }
    }

    /**
     * Get snapshot statistics.
     */
    suspend fun getSnapshotStats(snapshotPath: String): SnapshotStats? = withContext(Dispatchers.Default) {
        return@withContext try {
            nativeGetSnapshotStats(snapshotPath)
        } catch (e: Exception) {
            Log.e(TAG, "Error getting snapshot stats", e)
            null
        }
    }

    /**
     * Collect training data from various sources.
     */
    private suspend fun collectTrainingData(
        includeChat: Boolean,
        includeActivity: Boolean,
        onProgress: (Long, Long) -> Unit
    ): TrainingData? = withContext(Dispatchers.Default) {
        return@withContext try {
            var total = 0L
            var current = 0L

            if (includeChat) total += 40
            if (includeActivity) total += 60

            val chatData = if (includeChat) {
                onProgress(current, total)
                current += 40
                nativeGetChatData() ?: emptyList()
            } else {
                emptyList()
            }

            val activityData = if (includeActivity) {
                onProgress(current, total)
                current += 60
                nativeGetActivityData() ?: emptyList()
            } else {
                emptyList()
            }

            onProgress(total, total)

            TrainingData(
                timestamp = System.currentTimeMillis(),
                chatHistory = chatData,
                activityLogs = activityData
            )
        } catch (e: Exception) {
            Log.e(TAG, "Error collecting training data", e)
            null
        }
    }

    companion object {
        private const val TAG = "TdlExporter"
    }
}

data class TrainingData(
    val timestamp: Long,
    val chatHistory: List<ChatRecord>,
    val activityLogs: List<ActivityRecord>
)

data class ChatRecord(
    val id: String,
    val timestamp: Long,
    val role: String,
    val content: String,
    val model: String
)

data class ActivityRecord(
    val id: String,
    val timestamp: Long,
    val type: String,
    val description: String,
    val status: String
)

data class SnapshotInfo(
    val name: String,
    val path: String,
    val size: Long,
    val modified: Long
)

data class SnapshotStats(
    val name: String,
    val size: Long,
    val modified: Long,
    val chatRecordCount: Long,
    val activityRecordCount: Long,
    val quality: Float
)

// Native functions for TDL operations
private external fun nativeExportTdl(
    data: TrainingData,
    outputPath: String,
    name: String
): Boolean

private external fun nativeSyncSnapshot(
    snapshotPath: String,
    remoteUrl: String,
    onProgress: (Long, Long) -> Unit
): Boolean

private external fun nativeGetChatData(): List<ChatRecord>?

private external fun nativeGetActivityData(): List<ActivityRecord>?

private external fun nativeGetSnapshotStats(
    snapshotPath: String
): SnapshotStats?
