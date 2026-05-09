package ai.bonsai.buddy.di

import android.content.Context
import ai.bonsai.buddy.data.db.ActivityDao
import ai.bonsai.buddy.data.db.BonsaiDatabase
import ai.bonsai.buddy.data.db.ChatDao
import ai.bonsai.buddy.data.db.ModelDao
import ai.bonsai.buddy.data.db.ToolDao
import ai.bonsai.buddy.data.discovery.NsdDiscoveryManager
import ai.bonsai.buddy.data.logging.BonsaiLogger
import ai.bonsai.buddy.data.network.BonsaiApiClient
import ai.bonsai.buddy.data.repository.ChatRepository
import ai.bonsai.buddy.data.repository.mobile.ActivityRepository
import ai.bonsai.buddy.data.repository.mobile.ModelsRepository
import ai.bonsai.buddy.data.repository.mobile.ToolsRepository
import ai.bonsai.buddy.data.storage.SecureConfigStore
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
    fun provideApiClient(configStore: SecureConfigStore, logger: BonsaiLogger): BonsaiApiClient =
        BonsaiApiClient(configStore, logger)

    @Provides
    @Singleton
    fun provideDatabase(@ApplicationContext context: Context): BonsaiDatabase =
        Room.databaseBuilder(
            context,
            BonsaiDatabase::class.java,
            "bonsai_buddy.db"
        ).fallbackToDestructiveMigration().build()

    @Provides
    fun provideChatDao(db: BonsaiDatabase): ChatDao = db.chatDao()

    @Provides
    fun provideToolDao(db: BonsaiDatabase): ToolDao = db.toolDao()

    @Provides
    fun provideModelDao(db: BonsaiDatabase): ModelDao = db.modelDao()

    @Provides
    fun provideActivityDao(db: BonsaiDatabase): ActivityDao = db.activityDao()

    @Provides
    @Singleton
    fun provideChatRepository(dao: ChatDao): ChatRepository =
        ChatRepository(dao)

    @Provides
    @Singleton
    fun provideToolsRepository(apiClient: BonsaiApiClient, toolDao: ToolDao): ToolsRepository =
        ToolsRepository(apiClient, toolDao)

    @Provides
    @Singleton
    fun provideModelsRepository(apiClient: BonsaiApiClient, modelDao: ModelDao): ModelsRepository =
        ModelsRepository(apiClient, modelDao)

    @Provides
    @Singleton
    fun provideActivityRepository(apiClient: BonsaiApiClient, activityDao: ActivityDao): ActivityRepository =
        ActivityRepository(apiClient, activityDao)

    @Provides
    @Singleton
    fun provideNsdManager(@ApplicationContext context: Context): NsdDiscoveryManager =
        NsdDiscoveryManager(context)
}
