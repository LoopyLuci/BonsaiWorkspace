package ai.bonsai.buddy.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface ModelDao {
    @Query("SELECT * FROM models ORDER BY loaded DESC, name ASC")
    suspend fun getAll(): List<ModelEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertAll(items: List<ModelEntity>)

    @Query("DELETE FROM models")
    suspend fun clearAll()
}
