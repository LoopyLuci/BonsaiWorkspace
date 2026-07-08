package ai.omnisystem.buddy.data.repository.mobile

import ai.omnisystem.buddy.data.db.ActivityDao
import ai.omnisystem.buddy.data.db.ActivityEntity
import ai.omnisystem.buddy.data.network.OmnisystemApiClient
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ActivityRepository @Inject constructor(
    private val apiClient: OmnisystemApiClient,
    private val activityDao: ActivityDao
) {
    suspend fun refresh(): List<ActivityEntity> {
        val fresh = apiClient.fetchActivityEvents().getOrNull().orEmpty().map {
            ActivityEntity(
                id = it.id,
                type = it.type,
                message = it.message,
                level = it.level,
                timestamp = it.timestamp
            )
        }
        if (fresh.isNotEmpty()) {
            activityDao.upsertAll(fresh)
            return fresh
        }
        return activityDao.getAll()
    }

    suspend fun list(typeFilter: String?): List<ActivityEntity> {
        return if (typeFilter.isNullOrBlank() || typeFilter == "all") {
            activityDao.getAll()
        } else {
            activityDao.getByType(typeFilter)
        }
    }
}
