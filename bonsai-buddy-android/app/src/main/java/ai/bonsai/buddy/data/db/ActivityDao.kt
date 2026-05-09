package ai.bonsai.buddy.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface ActivityDao {
    @Query("SELECT * FROM activity_events ORDER BY timestamp DESC")
    suspend fun getAll(): List<ActivityEntity>

    @Query("SELECT * FROM activity_events WHERE type = :type ORDER BY timestamp DESC")
    suspend fun getByType(type: String): List<ActivityEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertAll(items: List<ActivityEntity>)

    @Query("DELETE FROM activity_events")
    suspend fun clearAll()
}
