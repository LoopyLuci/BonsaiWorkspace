package ai.bonsai.shared.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

@Database(
    entities = [
        ModelEntity::class,
        TokenEntity::class,
        PeerEntity::class,
        SettingsEntity::class,
        ChatHistoryEntity::class
    ],
    version = 1,
    exportSchema = false
)
abstract class BonsaiDatabase : RoomDatabase() {
    abstract fun bonsaiDao(): BonsaiDao

    companion object {
        @Volatile
        private var INSTANCE: BonsaiDatabase? = null

        fun getInstance(context: Context): BonsaiDatabase {
            return INSTANCE ?: synchronized(this) {
                Room.databaseBuilder(
                    context.applicationContext,
                    BonsaiDatabase::class.java,
                    "bonsai_db"
                )
                    .fallbackToDestructiveMigration()
                    .build()
                    .also { INSTANCE = it }
            }
        }
    }
}
