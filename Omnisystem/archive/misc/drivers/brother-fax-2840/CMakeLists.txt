cmake_minimum_required(VERSION 3.24)
project(BrotherFAXDriver
    VERSION 1.0.0
    DESCRIPTION "macOS DriverKit driver for Brother IntelliFAX 2840"
    LANGUAGES CXX
)

# ============================================================================
# Build Configuration
# ============================================================================

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_VISIBILITY_PRESET hidden)
set(CMAKE_VISIBILITY_INLINES_HIDDEN ON)

# Apple-specific settings
set(CMAKE_OSX_DEPLOYMENT_TARGET "11.0" CACHE STRING "Minimum macOS version")
set(CMAKE_OSX_ARCHITECTURES "arm64;x86_64" CACHE STRING "Supported architectures")

# ============================================================================
# DriverKit Extension Target
# ============================================================================

add_library(BrotherFAXDriver BUNDLE
    BrotherFAXDriver.cpp
    BrotherFAXDriver.hpp
)

# DriverKit framework location (try Xcode, then fall back to Xcode Command Line Tools)
if(NOT DEFINED DRIVERKIT_FRAMEWORK_PATH)
    execute_process(
        COMMAND xcrun --show-sdk-path
        OUTPUT_VARIABLE DRIVERKIT_SDK_PATH
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    set(DRIVERKIT_FRAMEWORK_PATH
        "${DRIVERKIT_SDK_PATH}/System/Library/Frameworks"
        CACHE PATH "Path to DriverKit frameworks"
    )
endif()

if(NOT DEFINED CMAKE_OSX_SYSROOT)
    execute_process(
        COMMAND xcrun --show-sdk-path
        OUTPUT_VARIABLE CMAKE_OSX_SYSROOT
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    set(CMAKE_OSX_SYSROOT "${CMAKE_OSX_SYSROOT}" CACHE PATH "OS X sysroot")
endif()

target_include_directories(BrotherFAXDriver PRIVATE
    ${DRIVERKIT_FRAMEWORK_PATH}/DriverKit.framework/Headers
)

target_link_directories(BrotherFAXDriver PRIVATE
    ${DRIVERKIT_FRAMEWORK_PATH}
)

target_link_libraries(BrotherFAXDriver PRIVATE
    DriverKit
)

target_compile_options(BrotherFAXDriver PRIVATE
    -isysroot
    ${CMAKE_OSX_SYSROOT}
)

# Compiler flags for DriverKit
target_compile_options(BrotherFAXDriver PRIVATE
    -fno-common
    -fno-strict-aliasing
    -fno-builtin
    -fPIC
    -mkernel
    -static
    -nostdinc
    -Werror=deprecated-declarations
    -Wall
    -Wextra
)

# Set bundle properties
set_target_properties(BrotherFAXDriver PROPERTIES
    BUNDLE TRUE
    BUNDLE_EXTENSION "dext"
    MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/Info.plist"
    MACOSX_BUNDLE_BUNDLE_NAME "BrotherFAXDriver"
    MACOSX_BUNDLE_BUNDLE_VERSION "1.0.0"
    MACOSX_BUNDLE_SHORT_VERSION_STRING "1.0"
    MACOSX_BUNDLE_COPYRIGHT "Copyright © 2026 Bonsai Ecosystem"
)

# ============================================================================
# Installation
# ============================================================================

install(
    TARGETS BrotherFAXDriver
    LIBRARY DESTINATION "Library/SystemExtensions"
    RUNTIME DESTINATION "Library/SystemExtensions"
)

# ============================================================================
# Code Signing
# ============================================================================

# For development, sign with ad-hoc identity
# For production, use proper code signing certificate
if(APPLE)
    set(CODESIGN_IDENTITY "-" CACHE STRING "Code signing identity")

    add_custom_command(TARGET BrotherFAXDriver POST_BUILD
        COMMAND /usr/bin/codesign
            -s "${CODESIGN_IDENTITY}"
            -f
            --entitlements "${CMAKE_CURRENT_SOURCE_DIR}/Entitlements.plist"
            "$<TARGET_BUNDLE_DIR:BrotherFAXDriver>"
        COMMENT "Signing DriverKit extension"
        VERBATIM
    )
endif()

# ============================================================================
# Documentation
# ============================================================================

message(STATUS "BrotherFAXDriver v${PROJECT_VERSION}")
message(STATUS "  macOS Deployment Target: ${CMAKE_OSX_DEPLOYMENT_TARGET}")
message(STATUS "  Architectures: ${CMAKE_OSX_ARCHITECTURES}")
message(STATUS "  Build Type: ${CMAKE_BUILD_TYPE}")
