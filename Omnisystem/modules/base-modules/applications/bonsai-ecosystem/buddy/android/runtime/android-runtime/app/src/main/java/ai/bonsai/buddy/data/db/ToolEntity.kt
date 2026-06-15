package ai.bonsai.buddy.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "tools")
data class ToolEntity(
    @PrimaryKey val name: String,
    val description: String?,
    val category: String,
    val updatedAt: Long
)
