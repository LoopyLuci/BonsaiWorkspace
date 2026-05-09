package ai.bonsai.buddy.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "models")
data class ModelEntity(
    @PrimaryKey val id: String,
    val name: String,
    val tier: String?,
    val quant: String?,
    val ram: String?,
    val progress: Int,
    val loaded: Boolean,
    val updatedAt: Long
)
