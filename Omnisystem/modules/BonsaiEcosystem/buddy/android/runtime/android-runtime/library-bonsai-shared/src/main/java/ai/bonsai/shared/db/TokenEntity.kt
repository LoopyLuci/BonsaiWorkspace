package ai.bonsai.shared.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "tokens")
data class TokenEntity(
    @PrimaryKey val peerId: String,
    val tokenData: ByteArray,
    val capabilities: String,
    val expiresAt: Long,
    val issuedAt: Long = System.currentTimeMillis(),
    val deviceName: String = "",
    val publicKey: String = ""
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is TokenEntity) return false
        if (peerId != other.peerId) return false
        if (!tokenData.contentEquals(other.tokenData)) return false
        return true
    }
    override fun hashCode(): Int {
        var result = peerId.hashCode()
        result = 31 * result + tokenData.contentHashCode()
        return result
    }
}
