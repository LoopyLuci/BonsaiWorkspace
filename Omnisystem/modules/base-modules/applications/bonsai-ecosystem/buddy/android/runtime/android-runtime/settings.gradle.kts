pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "BonsaiBuddyAndroid"

// Core
include(":library-bonsai-shared")

// Bonsai Buddy (AI Assistant)
include(":app")

// Phase 3: Remote Desktop
include(":app-remote")

// Standalone Apps (Phase 4)
include(":app-modelmanager")
include(":app-computedonor")
include(":app-nodecontroller")
include(":app-workspace")
include(":app-academy")
include(":app-extensions")

// Combination Apps (Phase 4g-4i)
include(":app-developer-suite")
include(":app-ai-power-user")
include(":app-sysadmin-console")
