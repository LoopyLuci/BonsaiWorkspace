package ai.bonsai.buddy.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

@Dao
interface ChatDao {
    @Query("SELECT * FROM chat_messages ORDER BY timestamp DESC LIMIT :limit OFFSET :offset")
    suspend fun getMessagesPage(limit: Int, offset: Int): List<ChatMessageEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insert(message: ChatMessageEntity): Long

    @Query("UPDATE chat_messages SET content = :content, timestamp = :timestamp WHERE id = :id")
    suspend fun updateContent(id: Long, content: String, timestamp: Long)

    @Query("DELETE FROM chat_messages")
    suspend fun clearAll()
}
