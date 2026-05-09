package ai.bonsai.buddy.di

import android.content.Context
import ai.bonsai.buddy.data.db.BonsaiDatabase
import ai.bonsai.buddy.data.db.ChatDao
import ai.bonsai.buddy.data.discovery.NsdDiscoveryManager
import ai.bonsai.buddy.data.network.BonsaiApiClient
import ai.bonsai.buddy.data.repository.ChatRepository
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
    fun provideApiClient(configStore: SecureConfigStore): BonsaiApiClient =
        BonsaiApiClient(configStore)

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
    @Singleton
    fun provideChatRepository(apiClient: BonsaiApiClient, dao: ChatDao): ChatRepository =
        ChatRepository(apiClient, dao)

    @Provides
    @Singleton
    fun provideNsdManager(@ApplicationContext context: Context): NsdDiscoveryManager =
        NsdDiscoveryManager(context)
}
