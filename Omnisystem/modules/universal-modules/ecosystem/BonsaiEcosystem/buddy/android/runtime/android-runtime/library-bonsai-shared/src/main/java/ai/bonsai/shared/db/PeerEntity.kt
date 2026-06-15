package ai.bonsai.shared.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "peers")
data class PeerEntity(
    @PrimaryKey val peerId: String,
    val name: String,
    val osType: String,
    val addresses: String,
    val lastSeen: Long = System.currentTimeMillis(),
    val online: Boolean = false,
    val publicKey: String = ""
)
