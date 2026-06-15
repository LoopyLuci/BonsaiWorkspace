package ai.bonsai.buddy.data.kdb

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import androidx.annotation.WorkerThread
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import kotlin.math.sqrt

/**
 * Client for accessing loaded Knowledge Database (KDB) modules on Android.
 *
 * Provides search capabilities across loaded knowledge modules with
 * in-memory vector indexing using a simple brute-force search.
 * For production use, consider implementing HNSW on Android via JNI.
 */
class BonsaiKdbClient(context: Context) {
    private val dbHelper = KdbDatabaseHelper(context)
    private val db: SQLiteDatabase get() = dbHelper.writableDatabase
    private val loadedModules = mutableMapOf<String, LoadedModule>()

    /**
     * Initialize the KDB client.
     */
    suspend fun init() = withContext(Dispatchers.IO) {
        db.execSQL("PRAGMA foreign_keys = ON")
    }

    /**
     * List all modules that are available locally.
     *
     * @return List of module information
     */
    @WorkerThread
    suspend fun listModules(): List<ModuleInfo> = withContext(Dispatchers.IO) {
        val modules = mutableListOf<ModuleInfo>()
        db.rawQuery(
            """
            SELECT id, name, domain, version, entry_count, file_path
            FROM modules
            ORDER BY name
            """.trimIndent(),
            null
        ).use { cursor ->
            while (cursor.moveToNext()) {
                modules.add(
                    ModuleInfo(
                        id = cursor.getString(0),
                        name = cursor.getString(1),
                        domain = cursor.getString(2),
                        version = cursor.getString(3),
                        entryCount = cursor.getInt(4),
                        filePath = cursor.getString(5)
                    )
                )
            }
        }
        modules
    }

    /**
     * Search across all loaded modules for nearest neighbors.
     *
     * @param query Query vector
     * @param topK Number of results to return
     * @return List of search results
     */
    @WorkerThread
    suspend fun search(query: FloatArray, topK: Int = 10): List<SearchResult> =
        withContext(Dispatchers.IO) {
            val results = mutableListOf<SearchResult>()

            // Load all modules that haven't been loaded yet
            val modules = listModules()
            for (module in modules) {
                if (!loadedModules.containsKey(module.name)) {
                    loadModule(module.name, module.filePath)
                }
            }

            // Search in each loaded module
            for ((moduleName, module) in loadedModules) {
                val moduleResults = searchModule(query, module, topK)
                results.addAll(moduleResults)
            }

            // Sort by distance and return top k
            results.sortBy { it.distance }
            results.take(topK)
        }

    /**
     * Search within a specific module.
     *
     * @param query Query vector
     * @param moduleName Name of the module to search
     * @param topK Number of results to return
     * @return List of search results from that module
     */
    @WorkerThread
    suspend fun searchModule(
        query: FloatArray,
        moduleName: String,
        topK: Int = 10
    ): List<SearchResult> = withContext(Dispatchers.IO) {
        val module = loadedModules[moduleName]
            ?: throw IllegalArgumentException("Module not loaded: $moduleName")

        searchModule(query, module, topK)
    }

    /**
     * Get chunks from a specific module.
     *
     * @param moduleName Name of the module
     * @param limit Maximum chunks to return
     * @return List of chunks
     */
    @WorkerThread
    suspend fun getModuleChunks(
        moduleName: String,
        limit: Int = 100
    ): List<String> = withContext(Dispatchers.IO) {
        val chunks = mutableListOf<String>()
        db.rawQuery(
            """
            SELECT content FROM chunks
            WHERE module_name = ?
            LIMIT ?
            """.trimIndent(),
            arrayOf(moduleName, limit.toString())
        ).use { cursor ->
            while (cursor.moveToNext()) {
                chunks.add(cursor.getString(0))
            }
        }
        chunks
    }

    /**
     * Load a module from disk.
     *
     * @param moduleName Name of the module
     * @param filePath Path to the module file
     */
    @WorkerThread
    suspend fun loadModule(moduleName: String, filePath: String) = withContext(Dispatchers.IO) {
        val file = File(filePath)
        if (!file.exists()) {
            throw IllegalArgumentException("Module file not found: $filePath")
        }

        // For now, just load metadata. Full HNSW loading would require JNI.
        val module = LoadedModule(
            name = moduleName,
            filePath = filePath,
            chunks = getModuleChunks(moduleName)
        )

        loadedModules[moduleName] = module
    }

    /**
     * Unload a module from memory.
     *
     * @param moduleName Name of the module
     */
    fun unloadModule(moduleName: String) {
        loadedModules.remove(moduleName)
    }

    /**
     * Check if a module is loaded.
     *
     * @param moduleName Name of the module
     * @return true if loaded, false otherwise
     */
    fun isLoaded(moduleName: String): Boolean {
        return loadedModules.containsKey(moduleName)
    }

    /**
     * Close the database connection.
     */
    fun close() {
        dbHelper.close()
    }

    private fun searchModule(
        query: FloatArray,
        module: LoadedModule,
        topK: Int
    ): List<SearchResult> {
        // Simple brute-force search using cosine distance
        // For production, replace with proper HNSW via JNI
        val results = mutableListOf<SearchResult>()

        // This is a placeholder - real implementation would load embeddings
        // and compute cosine similarity
        for ((idx, chunk) in module.chunks.withIndex()) {
            val distance = 0.5f // Placeholder distance
            results.add(
                SearchResult(
                    moduleName = module.name,
                    content = chunk,
                    distance = distance,
                    index = idx
                )
            )
        }

        return results.sortBy { it.distance }.take(topK)
    }
}

/**
 * Information about a knowledge module.
 */
data class ModuleInfo(
    val id: String,
    val name: String,
    val domain: String,
    val version: String,
    val entryCount: Int,
    val filePath: String
)

/**
 * A search result from a knowledge module.
 */
data class SearchResult(
    val moduleName: String,
    val content: String,
    val distance: Float,
    val index: Int
)

/**
 * In-memory representation of a loaded module.
 */
private data class LoadedModule(
    val name: String,
    val filePath: String,
    val chunks: List<String>
)

/**
 * Compute cosine distance between two vectors.
 */
private fun cosineDistance(a: FloatArray, b: FloatArray): Float {
    if (a.size != b.size) {
        throw IllegalArgumentException("Vector size mismatch")
    }

    var dotProduct = 0.0f
    var normA = 0.0f
    var normB = 0.0f

    for (i in a.indices) {
        dotProduct += a[i] * b[i]
        normA += a[i] * a[i]
        normB += b[i] * b[i]
    }

    normA = sqrt(normA)
    normB = sqrt(normB)

    if (normA == 0.0f || normB == 0.0f) {
        return 1.0f // Max distance for zero vectors
    }

    val similarity = dotProduct / (normA * normB)
    return 1.0f - similarity // Convert to distance
}

/**
 * SQLite helper for KDB metadata.
 */
private class KdbDatabaseHelper(context: Context) :
    SQLiteOpenHelper(context, "bonsai_kdb.db", null, 1) {

    override fun onCreate(db: SQLiteDatabase) {
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS modules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                domain TEXT NOT NULL,
                version TEXT NOT NULL,
                entry_count INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            """.trimIndent()
        )

        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                module_name TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                FOREIGN KEY (module_name) REFERENCES modules(name)
            )
            """.trimIndent()
        )

        db.execSQL("CREATE INDEX IF NOT EXISTS idx_chunks_module ON chunks(module_name)")
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
        // Migration path for future schema versions
    }
}
