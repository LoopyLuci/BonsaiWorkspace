package ai.bonsai.buddy.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "activity_events")
data class ActivityEntity(
    @PrimaryKey val id: String,
    val type: String,
    val message: String,
    val level: String?,
    val timestamp: Long
)
