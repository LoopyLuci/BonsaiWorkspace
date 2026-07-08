package ai.omnisystem.shared.db

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
abstract class OmnisystemDatabase : RoomDatabase() {
    abstract fun omnisystemDao(): OmnisystemDao

    companion object {
        @Volatile
        private var INSTANCE: OmnisystemDatabase? = null

        fun getInstance(context: Context): OmnisystemDatabase {
            return INSTANCE ?: synchronized(this) {
                Room.databaseBuilder(
                    context.applicationContext,
                    OmnisystemDatabase::class.java,
                    "omnisystem_db"
                )
                    .fallbackToDestructiveMigration()
                    .build()
                    .also { INSTANCE = it }
            }
        }
    }
}
