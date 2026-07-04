package ai.bonsai.buddy.data.tdl

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import androidx.annotation.WorkerThread
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.util.*

/**
 * Client for accessing the Bonsai Training Data Library (TDL) on Android.
 *
 * This wrapper manages a local SQLite copy of the TDL database,
 * synced from the desktop application.
 */
class BonsaiTdlClient(context: Context) {
    private val dbHelper = TdlDatabaseHelper(context)
    private val db: SQLiteDatabase get() = dbHelper.writableDatabase

    /**
     * Initialize the TDL client and database.
     */
    suspend fun init() = withContext(Dispatchers.IO) {
        db.execSQL("PRAGMA foreign_keys = ON")
    }

    /**
     * Get all versions in the library.
     *
     * @return List of version summaries
     */
    @WorkerThread
    suspend fun getVersions(): List<VersionInfo> = withContext(Dispatchers.IO) {
        val versions = mutableListOf<VersionInfo>()
        db.rawQuery(
            """
            SELECT id, version_string, example_count, created_by, created_at, avg_quality_score
            FROM versions
            ORDER BY created_at DESC
            """.trimIndent(),
            null
        ).use { cursor ->
            while (cursor.moveToNext()) {
                versions.add(
                    VersionInfo(
                        id = cursor.getString(0),
                        versionString = cursor.getString(1),
                        exampleCount = cursor.getInt(2),
                        createdBy = cursor.getString(3),
                        createdAt = cursor.getString(4),
                        avgQualityScore = cursor.getFloat(5)
                    )
                )
            }
        }
        versions
    }

    /**
     * Get examples from a specific version.
     *
     * @param versionId ID of the version
     * @param limit Maximum number of examples to return
     * @param offset Offset for pagination
     * @return List of examples
     */
    @WorkerThread
    suspend fun getExamples(
        versionId: String,
        limit: Int = 100,
        offset: Int = 0
    ): List<Example> = withContext(Dispatchers.IO) {
        val examples = mutableListOf<Example>()
        db.rawQuery(
            """
            SELECT id, content, metadata, quality_score, created_at, content_hash
            FROM examples
            WHERE version_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            """.trimIndent(),
            arrayOf(versionId, limit.toString(), offset.toString())
        ).use { cursor ->
            while (cursor.moveToNext()) {
                examples.add(
                    Example(
                        id = cursor.getString(0),
                        content = cursor.getString(1),
                        metadata = JSONObject(cursor.getString(2)),
                        qualityScore = cursor.getFloat(3),
                        createdAt = cursor.getString(4),
                        contentHash = cursor.getString(5)
                    )
                )
            }
        }
        examples
    }

    /**
     * Search for high-quality examples.
     *
     * @param minQuality Minimum quality score (0.0-1.0)
     * @param limit Maximum examples to return
     * @return List of high-quality examples
     */
    @WorkerThread
    suspend fun searchByQuality(
        minQuality: Float,
        limit: Int = 50
    ): List<Example> = withContext(Dispatchers.IO) {
        val examples = mutableListOf<Example>()
        db.rawQuery(
            """
            SELECT id, content, metadata, quality_score, created_at, content_hash
            FROM examples
            WHERE quality_score >= ?
            ORDER BY quality_score DESC, created_at DESC
            LIMIT ?
            """.trimIndent(),
            arrayOf(minQuality.toString(), limit.toString())
        ).use { cursor ->
            while (cursor.moveToNext()) {
                examples.add(
                    Example(
                        id = cursor.getString(0),
                        content = cursor.getString(1),
                        metadata = JSONObject(cursor.getString(2)),
                        qualityScore = cursor.getFloat(3),
                        createdAt = cursor.getString(4),
                        contentHash = cursor.getString(5)
                    )
                )
            }
        }
        examples
    }

    /**
     * Search for examples by domain and tags.
     *
     * @param domain Domain filter (e.g., "nlp", "code")
     * @param tags List of tags to match
     * @param limit Maximum examples to return
     * @return List of matching examples
     */
    @WorkerThread
    suspend fun searchByTags(
        domain: String? = null,
        tags: List<String> = emptyList(),
        limit: Int = 50
    ): List<Example> = withContext(Dispatchers.IO) {
        val examples = mutableListOf<Example>()

        var query =
            """
            SELECT DISTINCT e.id, e.content, e.metadata, e.quality_score, e.created_at, e.content_hash
            FROM examples e
            WHERE 1=1
            """.trimIndent()

        val args = mutableListOf<String>()

        if (domain != null) {
            query += " AND e.metadata LIKE ?"
            args.add("%\"domain\":\"$domain\"%")
        }

        if (tags.isNotEmpty()) {
            for (tag in tags) {
                query += " AND e.metadata LIKE ?"
                args.add("%$tag%")
            }
        }

        query += " ORDER BY e.quality_score DESC LIMIT ?"
        args.add(limit.toString())

        db.rawQuery(query, args.toTypedArray()).use { cursor ->
            while (cursor.moveToNext()) {
                examples.add(
                    Example(
                        id = cursor.getString(0),
                        content = cursor.getString(1),
                        metadata = JSONObject(cursor.getString(2)),
                        qualityScore = cursor.getFloat(3),
                        createdAt = cursor.getString(4),
                        contentHash = cursor.getString(5)
                    )
                )
            }
        }
        examples
    }

    /**
     * Export examples to a local file.
     *
     * @param versionId ID of the version to export
     * @param outputPath File path to write to
     * @param format Export format ("jsonl" or "json")
     */
    @WorkerThread
    suspend fun exportExamples(
        versionId: String,
        outputPath: String,
        format: String = "jsonl"
    ) = withContext(Dispatchers.IO) {
        val examples = getExamples(versionId, limit = Int.MAX_VALUE)
        val file = File(outputPath)
        file.parentFile?.mkdirs()

        when (format.lowercase()) {
            "jsonl" -> {
                file.writeText("")
                for (example in examples) {
                    val json = JSONObject().apply {
                        put("id", example.id)
                        put("content", example.content)
                        put("metadata", example.metadata)
                        put("quality_score", example.qualityScore)
                        put("created_at", example.createdAt)
                    }
                    file.appendText(json.toString() + "\n")
                }
            }
            "json" -> {
                val jsonArray = org.json.JSONArray()
                for (example in examples) {
                    val json = JSONObject().apply {
                        put("id", example.id)
                        put("content", example.content)
                        put("metadata", example.metadata)
                        put("quality_score", example.qualityScore)
                        put("created_at", example.createdAt)
                    }
                    jsonArray.put(json)
                }
                file.writeText(jsonArray.toString(2))
            }
            else -> throw IllegalArgumentException("Unsupported format: $format")
        }
    }

    /**
     * Close the database connection.
     */
    fun close() {
        dbHelper.close()
    }
}

/**
 * Version information from the TDL.
 */
data class VersionInfo(
    val id: String,
    val versionString: String,
    val exampleCount: Int,
    val createdBy: String,
    val createdAt: String,
    val avgQualityScore: Float
)

/**
 * Training example from the TDL.
 */
data class Example(
    val id: String,
    val content: String,
    val metadata: JSONObject,
    val qualityScore: Float,
    val createdAt: String,
    val contentHash: String
)

/**
 * SQLite helper for TDL database.
 */
private class TdlDatabaseHelper(context: Context) :
    SQLiteOpenHelper(context, "bonsai_tdl.db", null, 1) {

    override fun onCreate(db: SQLiteDatabase) {
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS versions (
                id TEXT PRIMARY KEY,
                version_string TEXT NOT NULL UNIQUE,
                example_count INTEGER NOT NULL DEFAULT 0,
                total_size_bytes INTEGER NOT NULL DEFAULT 0,
                created_by TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                avg_quality_score REAL NOT NULL DEFAULT 0.0,
                version_hash TEXT NOT NULL
            )
            """.trimIndent()
        )

        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS examples (
                id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                quality_score REAL NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                content_hash TEXT NOT NULL UNIQUE,
                content_size_bytes INTEGER NOT NULL,
                FOREIGN KEY (version_id) REFERENCES versions(id)
            )
            """.trimIndent()
        )

        db.execSQL("CREATE INDEX IF NOT EXISTS idx_examples_version_id ON examples(version_id)")
        db.execSQL("CREATE INDEX IF NOT EXISTS idx_examples_quality ON examples(quality_score DESC)")
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
        // Migration path for future schema versions
    }
}
