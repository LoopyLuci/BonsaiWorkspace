package ai.bonsai.shared.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "models")
data class ModelEntity(
    @PrimaryKey val id: String,
    val name: String,
    val version: String,
    val path: String,
    val quantization: String,
    val sizeBytes: Long,
    val lastUsed: Long = System.currentTimeMillis(),
    val isActive: Boolean = false,
    val downloadUrl: String = "",
    val checksumBlake3: String = ""
)
