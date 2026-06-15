package ai.bonsai.buddy.data.repository

import ai.bonsai.buddy.data.db.ChatDao
import ai.bonsai.buddy.data.db.ChatMessageEntity
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ChatRepository @Inject constructor(
    private val chatDao: ChatDao
) {
    suspend fun getMessagesPage(limit: Int, offset: Int): List<ChatMessageEntity> =
        chatDao.getMessagesPage(limit = limit, offset = offset)

    suspend fun appendMessage(role: String, content: String): Long {
        val now = System.currentTimeMillis()
        return chatDao.insert(
            ChatMessageEntity(
                role = role,
                content = content,
                timestamp = now
            )
        )
    }

    suspend fun updateMessage(id: Long, content: String) {
        chatDao.updateContent(id = id, content = content, timestamp = System.currentTimeMillis())
    }
}
