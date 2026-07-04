package ai.bonsai.buddy.data.repository.mobile

import ai.bonsai.buddy.data.db.ActivityDao
import ai.bonsai.buddy.data.db.ActivityEntity
import ai.bonsai.buddy.data.network.BonsaiApiClient
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ActivityRepository @Inject constructor(
    private val apiClient: BonsaiApiClient,
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
