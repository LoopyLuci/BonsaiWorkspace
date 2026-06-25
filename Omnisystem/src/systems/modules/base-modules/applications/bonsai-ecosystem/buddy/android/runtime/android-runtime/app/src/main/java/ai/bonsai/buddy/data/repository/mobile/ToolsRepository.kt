package ai.bonsai.buddy.data.repository.mobile

import ai.bonsai.buddy.data.db.ToolDao
import ai.bonsai.buddy.data.db.ToolEntity
import ai.bonsai.buddy.data.network.BonsaiApiClient
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ToolsRepository @Inject constructor(
    private val apiClient: BonsaiApiClient,
    private val toolDao: ToolDao
) {
    suspend fun listTools(): List<ToolEntity> {
        val cached = toolDao.getAll()
        val fresh = apiClient.fetchTools().getOrNull().orEmpty().map {
            ToolEntity(
                name = it.name,
                description = it.description,
                category = inferCategory(it.name),
                updatedAt = System.currentTimeMillis()
            )
        }
        if (fresh.isNotEmpty()) {
            toolDao.upsertAll(fresh)
            return fresh
        }
        return cached
    }

    suspend fun runTool(name: String, params: Map<String, String>): String {
        return apiClient.invokeTool(name, params).getOrElse {
            "Tool call failed: ${it.message ?: "unknown error"}"
        }
    }

    private fun inferCategory(name: String): String {
        val key = name.lowercase()
        return when {
            key.contains("file") -> "File"
            key.contains("sys") || key.contains("cpu") || key.contains("mem") -> "System"
            key.contains("rag") || key.contains("search") || key.contains("doc") -> "Knowledge"
            key.contains("mail") || key.contains("msg") || key.contains("notify") -> "Communication"
            key.contains("chart") || key.contains("plot") || key.contains("render") -> "Chart"
            else -> "General"
        }
    }
}
