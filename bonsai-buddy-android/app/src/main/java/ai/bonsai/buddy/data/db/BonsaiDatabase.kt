package ai.bonsai.buddy.data.db

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(
    entities = [ChatMessageEntity::class],
    version = 1,
    exportSchema = true
)
abstract class BonsaiDatabase : RoomDatabase() {
    abstract fun chatDao(): ChatDao
}
