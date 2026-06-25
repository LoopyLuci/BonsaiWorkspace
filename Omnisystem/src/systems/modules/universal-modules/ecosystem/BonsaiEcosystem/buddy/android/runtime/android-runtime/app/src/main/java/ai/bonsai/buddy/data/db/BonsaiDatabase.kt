package ai.bonsai.buddy.data.db

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(
    entities = [
        ChatMessageEntity::class,
        ToolEntity::class,
        ModelEntity::class,
        ActivityEntity::class
    ],
    version = 2,
    exportSchema = true
)
abstract class BonsaiDatabase : RoomDatabase() {
    abstract fun chatDao(): ChatDao
    abstract fun toolDao(): ToolDao
    abstract fun modelDao(): ModelDao
    abstract fun activityDao(): ActivityDao
}
