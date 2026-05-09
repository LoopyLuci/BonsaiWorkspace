package ai.bonsai.buddy.data.repository

import ai.bonsai.buddy.data.db.ChatDao
import ai.bonsai.buddy.data.db.ChatMessageEntity
import ai.bonsai.buddy.data.network.BonsaiApiClient
import kotlinx.coroutines.flow.Flow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ChatRepository @Inject constructor(
    private val apiClient: BonsaiApiClient,
    private val chatDao: ChatDao
) {
    fun streamMessages(): Flow<List<ChatMessageEntity>> = chatDao.streamMessages()

    suspend fun sendUserMessage(text: String): Result<Unit> = runCatching {
        val now = System.currentTimeMillis()
        chatDao.insert(
            ChatMessageEntity(
                role = "user",
                content = text,
                timestamp = now
            )
        )

        val response = apiClient.sendChatMessage(text).getOrElse {
            "Buddy is unreachable. Check LAN connection and desktop token."
        }

        chatDao.insert(
            ChatMessageEntity(
                role = "assistant",
                content = response,
                timestamp = now + 1
            )
        )
    }
}
