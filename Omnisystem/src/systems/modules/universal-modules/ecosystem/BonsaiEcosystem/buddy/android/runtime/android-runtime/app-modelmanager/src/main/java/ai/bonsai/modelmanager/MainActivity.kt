package ai.bonsai.modelmanager

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import ai.bonsai.modelmanager.ui.theme.BonsaiTheme
import ai.bonsai.modelmanager.ui.screens.ModelListScreen
import ai.bonsai.modelmanager.ui.screens.ModelDetailScreen
import ai.bonsai.modelmanager.ui.screens.ModelDownloaderScreen
import ai.bonsai.modelmanager.viewmodel.ModelManagerViewModel
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            BonsaiTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    val navController = rememberNavController()
                    val viewModel: ModelManagerViewModel = hiltViewModel()

                    NavHost(
                        navController = navController,
                        startDestination = "model_list"
                    ) {
                        composable("model_list") {
                            ModelListScreen(
                                viewModel = viewModel,
                                onModelClick = { modelId ->
                                    navController.navigate("model_detail/$modelId")
                                },
                                onDownloadClick = {
                                    navController.navigate("model_downloader")
                                }
                            )
                        }

                        composable(
                            "model_detail/{modelId}",
                            arguments = listOf(
                                navArgument("modelId") { type = NavType.StringType }
                            )
                        ) { backStackEntry ->
                            val modelId = backStackEntry.arguments?.getString("modelId") ?: return@composable
                            ModelDetailScreen(
                                modelId = modelId,
                                viewModel = viewModel,
                                onBack = { navController.popBackStack() },
                                onTest = {
                                    // Open Bonsai Buddy with model active
                                    // For now, just show a message
                                }
                            )
                        }

                        composable("model_downloader") {
                            ModelDownloaderScreen(
                                viewModel = viewModel,
                                onBack = { navController.popBackStack() },
                                onDownloadComplete = {
                                    navController.popBackStack()
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}
