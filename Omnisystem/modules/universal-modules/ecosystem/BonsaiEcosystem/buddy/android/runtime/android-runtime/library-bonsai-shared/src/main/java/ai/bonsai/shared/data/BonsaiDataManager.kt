package ai.bonsai.shared.data

import android.content.Context
import ai.bonsai.shared.db.BonsaiDatabase
import ai.bonsai.shared.db.*

class BonsaiDataManager(context: Context) {
    private val database = BonsaiDatabase.getInstance(context)
    private val dao = database.bonsaiDao()

    suspend fun getAllModels(): List<ModelEntity> = dao.getAllModels()
    suspend fun getActiveModel(): ModelEntity? = dao.getActiveModel()
    suspend fun setActiveModel(id: String) = dao.setActiveModel(id)
    suspend fun addModel(model: ModelEntity) = dao.insertModel(model)

    suspend fun getToken(peerId: String): TokenEntity? = dao.getToken(peerId)
    suspend fun getAllTokens(): List<TokenEntity> = dao.getAllTokens()
    suspend fun addToken(token: TokenEntity) = dao.insertToken(token)
    suspend fun removeExpiredTokens() = dao.deleteExpiredTokens(System.currentTimeMillis())

    suspend fun getAllPeers(): List<PeerEntity> = dao.getAllPeers()
    suspend fun getOnlinePeers(): List<PeerEntity> = dao.getOnlinePeers()
    suspend fun addPeer(peer: PeerEntity) = dao.insertPeer(peer)
    suspend fun updatePeer(peer: PeerEntity) = dao.updatePeer(peer)

    suspend fun getSetting(key: String): String? = dao.getSetting(key)?.value
    suspend fun setSetting(key: String, value: String) = dao.insertSetting(SettingsEntity(key, value))

    suspend fun getChatHistory(conversationId: String): List<ChatHistoryEntity> = dao.getChatHistory(conversationId)
    suspend fun addChatMessage(message: ChatHistoryEntity) = dao.insertChatMessage(message)
    suspend fun clearConversation(conversationId: String) = dao.clearConversation(conversationId)
}
