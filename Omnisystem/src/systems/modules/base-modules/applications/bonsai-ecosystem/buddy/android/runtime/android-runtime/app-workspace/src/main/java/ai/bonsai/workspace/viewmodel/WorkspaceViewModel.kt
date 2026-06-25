package ai.bonsai.workspace.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import android.util.Log
import java.util.*
import javax.inject.Inject
import ai.bonsai.workspace.ui.ProjectSummary

private const val TAG = "WorkspaceViewModel"

@HiltViewModel
class WorkspaceViewModel @Inject constructor() : ViewModel() {

    private val _projects = MutableStateFlow<List<ProjectSummary>>(emptyList())
    val projects: StateFlow<List<ProjectSummary>> = _projects.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _selectedProject = MutableStateFlow<String?>(null)
    val selectedProject: StateFlow<String?> = _selectedProject.asStateFlow()

    init {
        loadProjects()
    }

    fun loadProjects() {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            try {
                // Simulate loading projects from storage
                val mockProjects = listOf(
                    ProjectSummary(
                        id = UUID.randomUUID().toString(),
                        name = "Sample Project 1",
                        description = "A test project for development",
                        lastModified = System.currentTimeMillis(),
                        fileCount = 5
                    ),
                    ProjectSummary(
                        id = UUID.randomUUID().toString(),
                        name = "Sample Project 2",
                        description = "Another test project",
                        lastModified = System.currentTimeMillis() - 86400000,
                        fileCount = 12
                    )
                )
                _projects.value = mockProjects
                Log.i(TAG, "Loaded ${mockProjects.size} projects")
            } catch (e: Exception) {
                Log.e(TAG, "Error loading projects", e)
                _error.value = "Failed to load projects: ${e.message}"
            } finally {
                _isLoading.value = false
            }
        }
    }

    fun createProject(name: String) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            try {
                val newProject = ProjectSummary(
                    id = UUID.randomUUID().toString(),
                    name = name,
                    description = "Created at ${java.text.SimpleDateFormat("MMM d, yyyy", Locale.getDefault()).format(Date())}",
                    lastModified = System.currentTimeMillis(),
                    fileCount = 0
                )
                _projects.value = _projects.value + newProject
                Log.i(TAG, "Created project: $name")
            } catch (e: Exception) {
                Log.e(TAG, "Error creating project", e)
                _error.value = "Failed to create project: ${e.message}"
            } finally {
                _isLoading.value = false
            }
        }
    }

    fun selectProject(projectId: String) {
        _selectedProject.value = projectId
        Log.i(TAG, "Selected project: $projectId")
    }

    fun deleteProject(projectId: String) {
        viewModelScope.launch {
            _error.value = null
            try {
                _projects.value = _projects.value.filterNot { it.id == projectId }
                Log.i(TAG, "Deleted project: $projectId")
            } catch (e: Exception) {
                Log.e(TAG, "Error deleting project", e)
                _error.value = "Failed to delete project: ${e.message}"
            }
        }
    }

    fun clearError() {
        _error.value = null
    }
}
