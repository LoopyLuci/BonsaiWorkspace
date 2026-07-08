package ai.omnisystem.buddy.di

import android.content.Context
import ai.omnisystem.buddy.data.db.ActivityDao
import ai.omnisystem.buddy.data.db.OmnisystemDatabase
import ai.omnisystem.buddy.data.db.ChatDao
import ai.omnisystem.buddy.data.db.ModelDao
import ai.omnisystem.buddy.data.db.ToolDao
import ai.omnisystem.buddy.data.discovery.NsdDiscoveryManager
import ai.omnisystem.buddy.data.logging.OmnisystemLogger
import ai.omnisystem.buddy.data.network.OmnisystemApiClient
import ai.omnisystem.buddy.data.repository.ChatRepository
import ai.omnisystem.buddy.data.repository.mobile.ActivityRepository
import ai.omnisystem.buddy.data.repository.mobile.ModelsRepository
import ai.omnisystem.buddy.data.repository.mobile.ToolsRepository
import ai.omnisystem.buddy.data.storage.SecureConfigStore
import androidx.room.Room
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object AppModule {
    @Provides
    @Singleton
    fun provideSecureConfigStore(@ApplicationContext context: Context): SecureConfigStore =
        SecureConfigStore(context)

    @Provides
    @Singleton
    fun provideApiClient(configStore: SecureConfigStore, logger: OmnisystemLogger): OmnisystemApiClient =
        OmnisystemApiClient(configStore, logger)

    @Provides
    @Singleton
    fun provideDatabase(@ApplicationContext context: Context): OmnisystemDatabase =
        Room.databaseBuilder(
            context,
            OmnisystemDatabase::class.java,
            "omnisystem_buddy.db"
        ).fallbackToDestructiveMigration().build()

    @Provides
    fun provideChatDao(db: OmnisystemDatabase): ChatDao = db.chatDao()

    @Provides
    fun provideToolDao(db: OmnisystemDatabase): ToolDao = db.toolDao()

    @Provides
    fun provideModelDao(db: OmnisystemDatabase): ModelDao = db.modelDao()

    @Provides
    fun provideActivityDao(db: OmnisystemDatabase): ActivityDao = db.activityDao()

    @Provides
    @Singleton
    fun provideChatRepository(dao: ChatDao): ChatRepository =
        ChatRepository(dao)

    @Provides
    @Singleton
    fun provideToolsRepository(apiClient: OmnisystemApiClient, toolDao: ToolDao): ToolsRepository =
        ToolsRepository(apiClient, toolDao)

    @Provides
    @Singleton
    fun provideModelsRepository(apiClient: OmnisystemApiClient, modelDao: ModelDao): ModelsRepository =
        ModelsRepository(apiClient, modelDao)

    @Provides
    @Singleton
    fun provideActivityRepository(apiClient: OmnisystemApiClient, activityDao: ActivityDao): ActivityRepository =
        ActivityRepository(apiClient, activityDao)

    @Provides
    @Singleton
    fun provideNsdManager(@ApplicationContext context: Context): NsdDiscoveryManager =
        NsdDiscoveryManager(context)
}
