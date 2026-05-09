package ai.bonsai.buddy.data.repository.mobile

import ai.bonsai.buddy.data.db.ModelDao
import ai.bonsai.buddy.data.db.ModelEntity
import ai.bonsai.buddy.data.network.BonsaiApiClient
import ai.bonsai.buddy.data.network.InferenceMode
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ModelsRepository @Inject constructor(
    private val apiClient: BonsaiApiClient,
    private val modelDao: ModelDao
) {
    suspend fun listModels(): List<ModelEntity> {
        val cached = modelDao.getAll()
        val fresh = apiClient.fetchModels().getOrNull().orEmpty().map {
            ModelEntity(
                id = it.id,
                name = it.name,
                tier = it.tier,
                quant = it.quant,
                ram = it.ram,
                progress = it.progress ?: 0,
                loaded = it.loaded,
                updatedAt = System.currentTimeMillis()
            )
        }
        if (fresh.isNotEmpty()) {
            modelDao.upsertAll(fresh)
            return fresh
        }
        return cached
    }

    suspend fun load(modelId: String) {
        apiClient.loadModel(modelId)
        listModels()
    }

    suspend fun unload(modelId: String) {
        apiClient.unloadModel(modelId)
        listModels()
    }

    suspend fun setMode(mode: InferenceMode) {
        apiClient.setInferenceMode(mode)
    }
}
