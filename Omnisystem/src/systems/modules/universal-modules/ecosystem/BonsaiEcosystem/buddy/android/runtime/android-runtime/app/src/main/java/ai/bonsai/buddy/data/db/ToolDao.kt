package ai.bonsai.buddy.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface ToolDao {
    @Query("SELECT * FROM tools ORDER BY name ASC")
    suspend fun getAll(): List<ToolEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertAll(items: List<ToolEntity>)

    @Query("DELETE FROM tools")
    suspend fun clearAll()
}
