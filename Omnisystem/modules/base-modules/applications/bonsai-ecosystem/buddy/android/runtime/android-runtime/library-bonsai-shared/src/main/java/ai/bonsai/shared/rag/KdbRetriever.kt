package ai.bonsai.shared.rag

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Retrieves relevant documents from KDB (Knowledge Database) for RAG-augmented inference.
 * Uses vector similarity search to find contextually relevant information.
 */
class KdbRetriever(private val context: Context) {
    private val kdbDir = File(context.getExternalFilesDir(null), "kdb")

    init {
        kdbDir.mkdirs()
    }

    /**
     * Search KDB for relevant documents matching the query.
     *
     * @param query User query or prompt to search for
     * @param topK Number of documents to retrieve (default 5)
     * @param minSimilarity Minimum similarity score (0.0-1.0)
     * @return List of relevant documents with metadata
     */
    suspend fun retrieveDocuments(
        query: String,
        topK: Int = 5,
        minSimilarity: Float = 0.5f
    ): List<RetrievedDocument> = withContext(Dispatchers.Default) {
        return@withContext try {
            // Encode query to vector
            val queryVector = encodeQuery(query) ?: return@withContext emptyList()

            // Search KDB for similar vectors
            val results = nativeSearchKdb(
                queryVector,
                topK,
                minSimilarity
            )

            results.map { (docId, score, content, metadata) ->
                RetrievedDocument(
                    id = docId,
                    content = content,
                    similarity = score,
                    source = metadata["source"] as? String ?: "unknown",
                    type = metadata["type"] as? String ?: "document"
                )
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error retrieving documents", e)
            emptyList()
        }
    }

    /**
     * Format retrieved documents as context for model input.
     *
     * @param documents Retrieved documents
     * @param maxTokens Maximum tokens to include
     * @return Formatted context string
     */
    fun formatContext(
        documents: List<RetrievedDocument>,
        maxTokens: Int = 2048
    ): String {
        if (documents.isEmpty()) return ""

        val sb = StringBuilder()
        sb.append("Context from Knowledge Base:\n\n")

        var tokenCount = 0
        for (doc in documents) {
            if (tokenCount >= maxTokens) break

            val docText = """
                [${doc.source}] (similarity: ${String.format("%.2f", doc.similarity)})
                ${doc.content}

            """.trimIndent()

            val tokens = estimateTokens(docText)
            if (tokenCount + tokens <= maxTokens) {
                sb.append(docText)
                tokenCount += tokens
            }
        }

        return sb.toString()
    }

    /**
     * Index a new document into KDB.
     *
     * @param document Document content to index
     * @param metadata Metadata (source, type, tags, etc.)
     * @return Document ID or null if indexing failed
     */
    suspend fun indexDocument(
        document: String,
        metadata: Map<String, String>
    ): String? = withContext(Dispatchers.Default) {
        return@withContext try {
            // Encode document to vector
            val vector = encodeDocument(document) ?: return@withContext null

            // Index in KDB
            val docId = nativeIndexDocument(
                vector,
                document,
                metadata
            )

            if (docId != null) {
                Log.d(TAG, "Document indexed: $docId")
            }
            docId
        } catch (e: Exception) {
            Log.e(TAG, "Error indexing document", e)
            null
        }
    }

    /**
     * Clear KDB contents (useful for resetting).
     */
    suspend fun clearKdb() = withContext(Dispatchers.Default) {
        try {
            nativeClearKdb()
            Log.d(TAG, "KDB cleared")
        } catch (e: Exception) {
            Log.e(TAG, "Error clearing KDB", e)
        }
    }

    /**
     * Get KDB statistics.
     */
    suspend fun getStats(): KdbStats? = withContext(Dispatchers.Default) {
        return@withContext try {
            nativeGetKdbStats()
        } catch (e: Exception) {
            Log.e(TAG, "Error getting KDB stats", e)
            null
        }
    }

    /**
     * Encode query to vector using model embeddings.
     */
    private suspend fun encodeQuery(query: String): FloatArray? = withContext(Dispatchers.Default) {
        return@withContext try {
            nativeEncodeText(query, "query")
        } catch (e: Exception) {
            Log.e(TAG, "Error encoding query", e)
            null
        }
    }

    /**
     * Encode document to vector.
     */
    private suspend fun encodeDocument(document: String): FloatArray? = withContext(Dispatchers.Default) {
        return@withContext try {
            nativeEncodeText(document, "document")
        } catch (e: Exception) {
            Log.e(TAG, "Error encoding document", e)
            null
        }
    }

    /**
     * Estimate token count for text (approximation: ~4 chars per token).
     */
    private fun estimateTokens(text: String): Int = (text.length / 4) + 1

    companion object {
        private const val TAG = "KdbRetriever"
    }
}

data class RetrievedDocument(
    val id: String,
    val content: String,
    val similarity: Float,
    val source: String,
    val type: String
)

data class KdbStats(
    val documentCount: Long,
    val totalSize: Long,
    val indexSize: Long,
    val lastUpdated: Long
)

// Native functions for KDB operations
private external fun nativeSearchKdb(
    queryVector: FloatArray,
    topK: Int,
    minSimilarity: Float
): List<Tuple4<String, Float, String, Map<String, Any>>>

private external fun nativeIndexDocument(
    vector: FloatArray,
    content: String,
    metadata: Map<String, String>
): String?

private external fun nativeEncodeText(
    text: String,
    type: String
): FloatArray?

private external fun nativeClearKdb()

private external fun nativeGetKdbStats(): KdbStats?

// Helper class for tuple return from native code
data class Tuple4<A, B, C, D>(val first: A, val second: B, val third: C, val fourth: D)
