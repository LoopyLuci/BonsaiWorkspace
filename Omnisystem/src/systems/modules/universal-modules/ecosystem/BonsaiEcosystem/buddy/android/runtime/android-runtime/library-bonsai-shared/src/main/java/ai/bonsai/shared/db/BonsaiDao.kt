package ai.bonsai.shared.db

import androidx.room.*

@Dao
interface BonsaiDao {
    // Models
    @Query("SELECT * FROM models")
    suspend fun getAllModels(): List<ModelEntity>

    @Query("SELECT * FROM models WHERE id = :id")
    suspend fun getModel(id: String): ModelEntity?

    @Query("SELECT * FROM models WHERE isActive = 1")
    suspend fun getActiveModel(): ModelEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertModel(model: ModelEntity)

    @Delete
    suspend fun deleteModel(model: ModelEntity)

    @Query("UPDATE models SET isActive = 0")
    suspend fun deactivateAllModels()

    @Query("UPDATE models SET isActive = 1 WHERE id = :id")
    suspend fun setActiveModel(id: String)

    // Tokens
    @Query("SELECT * FROM tokens WHERE peerId = :peerId")
    suspend fun getToken(peerId: String): TokenEntity?

    @Query("SELECT * FROM tokens")
    suspend fun getAllTokens(): List<TokenEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertToken(token: TokenEntity)

    @Delete
    suspend fun deleteToken(token: TokenEntity)

    @Query("DELETE FROM tokens WHERE expiresAt < :currentTime")
    suspend fun deleteExpiredTokens(currentTime: Long)

    // Peers
    @Query("SELECT * FROM peers")
    suspend fun getAllPeers(): List<PeerEntity>

    @Query("SELECT * FROM peers WHERE peerId = :peerId")
    suspend fun getPeer(peerId: String): PeerEntity?

    @Query("SELECT * FROM peers WHERE online = 1")
    suspend fun getOnlinePeers(): List<PeerEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertPeer(peer: PeerEntity)

    @Update
    suspend fun updatePeer(peer: PeerEntity)

    @Delete
    suspend fun deletePeer(peer: PeerEntity)

    // Settings
    @Query("SELECT * FROM settings WHERE key = :key")
    suspend fun getSetting(key: String): SettingsEntity?

    @Query("SELECT * FROM settings")
    suspend fun getAllSettings(): List<SettingsEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertSetting(setting: SettingsEntity)

    @Query("DELETE FROM settings WHERE key = :key")
    suspend fun deleteSetting(key: String)

    // Chat History
    @Query("SELECT * FROM chat_history WHERE conversationId = :conversationId ORDER BY timestamp ASC")
    suspend fun getChatHistory(conversationId: String): List<ChatHistoryEntity>

    @Insert
    suspend fun insertChatMessage(message: ChatHistoryEntity)

    @Query("DELETE FROM chat_history WHERE conversationId = :conversationId")
    suspend fun clearConversation(conversationId: String)
}
