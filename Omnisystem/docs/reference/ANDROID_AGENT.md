# Bonsai Android Agent - Kotlin Implementation Guide

## Overview

The Bonsai Android Agent is a lightweight companion service running on Android devices that enables the Desktop Bridge to control the device. This document provides the implementation skeleton and architecture for the Kotlin agent.

## Project Structure

```
android-agent/
├── app/
│  ├── src/
│  │  ├── main/
│  │  │  ├── java/com/bonsai/agent/
│  │  │  │  ├── MainActivity.kt              # Main entry point
│  │  │  │  ├── BonsaiService.kt             # Foreground service
│  │  │  │  ├── connection/
│  │  │  │  │  ├── NoiseProtocol.kt          # Encryption handshake
│  │  │  │  │  ├── SessionKey.kt             # Session management
│  │  │  │  │  └── DeviceIdentity.kt         # PKI
│  │  │  │  ├── screen/
│  │  │  │  │  ├── ScreenEncoder.kt          # H.264/H.265 encoder
│  │  │  │  │  └── ScreenCapture.kt          # SurfaceFlinger integration
│  │  │  │  ├── input/
│  │  │  │  │  ├── InputHandler.kt           # Touch/Keyboard events
│  │  │  │  │  └── GestureFactory.kt         # Gesture synthesis
│  │  │  │  ├── file/
│  │  │  │  │  ├── FileSyncService.kt        # Bidirectional sync
│  │  │  │  │  └── DeltaCalculator.kt        # Incremental updates
│  │  │  │  ├── app/
│  │  │  │  │  ├── AppManager.kt             # APK installation
│  │  │  │  │  └─ HotReloader.kt             # Hot reload support
│  │  │  │  ├── sensor/
│  │  │  │  │  ├── SensorProvider.kt         # GPS, IMU, etc.
│  │  │  │  │  └── DeviceMonitor.kt          # Battery, temp
│  │  │  │  ├── capability/
│  │  │  │  │  ├── CapabilityChecker.kt      # Token validation
│  │  │  │  │  └── CapabilityToken.kt        # Token data class
│  │  │  │  ├── telemetry/
│  │  │  │  │  ├── TelemetryReporter.kt      # Event logging
│  │  │  │  │  └── Metrics.kt                # Performance metrics
│  │  │  │  └── BonsaiAgent.kt               # Main coordinator
│  │  │  ├── AndroidManifest.xml
│  │  │  └── res/
│  │  │     ├── layout/
│  │  │     └── values/
│  │  └── androidTest/
│  │     └── ExampleInstrumentedTest.kt
│  └── build.gradle.kts
├── gradle/
│  └── wrapper/
├── settings.gradle.kts
└── README.md
```

## Core Implementation Skeleton

### 1. MainActivity.kt

```kotlin
package com.bonsai.agent

import android.content.Intent
import android.os.Build
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch

class MainActivity : AppCompatActivity() {
    private val scope = MainScope()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        // Start Bonsai Agent service
        val intent = Intent(this, BonsaiService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
    }
}
```

### 2. BonsaiService.kt

```kotlin
package com.bonsai.agent

import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.bonsai.agent.connection.NoiseProtocol
import com.bonsai.agent.screen.ScreenCapture
import com.bonsai.agent.input.InputHandler
import com.bonsai.agent.file.FileSyncService
import com.bonsai.agent.telemetry.TelemetryReporter
import kotlinx.coroutines.*
import java.net.ServerSocket

class BonsaiService : Service() {
    private val scope = CoroutineScope(Dispatchers.Default + Job())
    private val binder = BonsaiBinder()

    private lateinit var agent: BonsaiAgent
    private var serverSocket: ServerSocket? = null

    inner class BonsaiBinder : Binder() {
        fun getService(): BonsaiService = this@BonsaiService
    }

    override fun onCreate() {
        super.onCreate()

        // Create notification for foreground service
        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Bonsai Agent")
            .setContentText("Running device control service")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .build()

        startForeground(NOTIFICATION_ID, notification)

        // Initialize agent
        agent = BonsaiAgent(applicationContext)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        scope.launch {
            startBridgeListener()
        }
        return START_STICKY
    }

    private suspend fun startBridgeListener() {
        try {
            serverSocket = ServerSocket(BRIDGE_PORT)
            while (isActive) {
                val socket = serverSocket?.accept() ?: break
                scope.launch {
                    agent.handleConnection(socket)
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
        serverSocket?.close()
    }

    override fun onBind(intent: Intent?): IBinder = binder

    companion object {
        private const val CHANNEL_ID = "bonsai_channel"
        private const val NOTIFICATION_ID = 1
        private const val BRIDGE_PORT = 5037
    }
}
```

### 3. BonsaiAgent.kt (Main Coordinator)

```kotlin
package com.bonsai.agent

import android.content.Context
import com.bonsai.agent.connection.SessionKey
import com.bonsai.agent.screen.ScreenCapture
import com.bonsai.agent.screen.ScreenEncoder
import com.bonsai.agent.input.InputHandler
import com.bonsai.agent.file.FileSyncService
import com.bonsai.agent.capability.CapabilityChecker
import com.bonsai.agent.telemetry.TelemetryReporter
import kotlinx.coroutines.*
import java.net.Socket
import javax.crypto.Cipher

class BonsaiAgent(private val context: Context) {
    private val scope = CoroutineScope(Dispatchers.Default + Job())

    private val screenCapture = ScreenCapture(context)
    private val screenEncoder = ScreenEncoder()
    private val inputHandler = InputHandler(context)
    private val fileSyncService = FileSyncService(context)
    private val capabilityChecker = CapabilityChecker()
    private val telemetry = TelemetryReporter()

    suspend fun handleConnection(socket: Socket) {
        try {
            // Perform Noise protocol handshake
            val sessionKey = performHandshake(socket)

            // Main event loop
            val reader = socket.inputStream.bufferedReader()
            val writer = socket.outputStream.bufferedWriter()

            while (isActive) {
                val message = reader.readLine() ?: break
                val decrypted = sessionKey.decrypt(message)

                scope.launch {
                    handleMessage(decrypted, sessionKey, writer)
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        } finally {
            socket.close()
        }
    }

    private suspend fun performHandshake(socket: Socket): SessionKey {
        // Implement Noise protocol IK pattern
        // 1. Receive ephemeral from bridge
        // 2. Generate ephemeral response
        // 3. Perform DH exchanges
        // 4. Derive session key
        return SessionKey.generateNew()
    }

    private suspend fun handleMessage(
        message: String,
        sessionKey: SessionKey,
        writer: java.io.BufferedWriter
    ) {
        val json = parseJson(message)
        val type = json["type"] as? String ?: return

        when (type) {
            "screen_request" -> handleScreenRequest(sessionKey, writer)
            "input_event" -> handleInputEvent(json, sessionKey)
            "file_sync" -> handleFileSync(json, sessionKey)
            "app_install" -> handleAppInstall(json, sessionKey)
            "capability_check" -> handleCapabilityCheck(json, sessionKey, writer)
            else -> telemetry.logUnknownMessage(type)
        }
    }

    private suspend fun handleScreenRequest(
        sessionKey: SessionKey,
        writer: java.io.BufferedWriter
    ) {
        val frame = screenCapture.captureFrame()
        val encoded = screenEncoder.encode(frame)
        val response = createFrameMessage(encoded)
        val encrypted = sessionKey.encrypt(response)
        writer.write(encrypted)
        writer.flush()
    }

    private suspend fun handleInputEvent(
        message: Map<String, Any>,
        sessionKey: SessionKey
    ) {
        val inputType = message["input_type"] as? String ?: return
        
        // Check capability
        val token = message["capability_token"] as? String
        if (token != null && !capabilityChecker.verify(token)) {
            telemetry.logCapabilityDenied(inputType)
            return
        }

        when (inputType) {
            "touch" -> {
                val x = (message["x"] as? Number)?.toFloat() ?: return
                val y = (message["y"] as? Number)?.toFloat() ?: return
                val action = (message["action"] as? String) ?: "down"
                inputHandler.injectTouch(x, y, action)
            }
            "keyboard" -> {
                val keyCode = (message["key_code"] as? Number)?.toInt() ?: return
                val pressed = message["pressed"] as? Boolean ?: true
                inputHandler.injectKey(keyCode, pressed)
            }
            "text" -> {
                val text = message["text"] as? String ?: return
                inputHandler.injectText(text)
            }
        }

        telemetry.logInputEvent(inputType)
    }

    private suspend fun handleFileSync(
        message: Map<String, Any>,
        sessionKey: SessionKey
    ) {
        val direction = message["direction"] as? String ?: return
        fileSyncService.sync(direction)
        telemetry.logFileSyncStart(direction)
    }

    private suspend fun handleAppInstall(
        message: Map<String, Any>,
        sessionKey: SessionKey
    ) {
        val apkUrl = message["apk_url"] as? String ?: return
        // Implement APK installation
        telemetry.logAppInstallStart(apkUrl)
    }

    private suspend fun handleCapabilityCheck(
        message: Map<String, Any>,
        sessionKey: SessionKey,
        writer: java.io.BufferedWriter
    ) {
        val token = message["token"] as? String ?: return
        val capability = message["capability"] as? String ?: return

        val isValid = capabilityChecker.verify(token, capability)
        val response = """{"valid": $isValid}"""
        val encrypted = sessionKey.encrypt(response)
        
        writer.write(encrypted)
        writer.flush()
    }

    private fun createFrameMessage(encoded: ByteArray): String {
        // Convert to JSON message format
        return """{"type":"screen_frame","data":"${encoded.joinToString("") { it.toInt().toString() }}"}"""
    }

    private fun parseJson(json: String): Map<String, Any> {
        // Simple JSON parsing (use proper library in production)
        return emptyMap()
    }
}
```

### 4. Screen Capture & Encoding

```kotlin
// screen/ScreenCapture.kt
package com.bonsai.agent.screen

import android.content.Context
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.view.Display
import android.view.Surface
import kotlinx.coroutines.*

data class ScreenFrame(
    val timestamp: Long,
    val width: Int,
    val height: Int,
    val data: ByteArray
)

class ScreenCapture(context: Context) {
    private val displayManager = context.getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
    private var lastFrame: ScreenFrame? = null

    suspend fun captureFrame(): ScreenFrame = withContext(Dispatchers.Default) {
        // Use MediaProjection API to get screen data
        val display = displayManager.displays.firstOrNull() ?: error("No display found")
        
        ScreenFrame(
            timestamp = System.currentTimeMillis(),
            width = display.width,
            height = display.height,
            data = ByteArray(0) // Would be actual frame data
        )
    }
}

// screen/ScreenEncoder.kt
package com.bonsai.agent.screen

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import java.nio.ByteBuffer

class ScreenEncoder(
    private val width: Int = 1080,
    private val height: Int = 2400,
    private val fps: Int = 60,
    private val bitrate: Int = 5000000, // 5 Mbps
    private val codec: String = MediaFormat.MIMETYPE_VIDEO_HEVC // H.265
) {
    private var encoder: MediaCodec? = null
    private var frameCount = 0L

    fun encode(frame: ScreenFrame): ByteArray {
        if (encoder == null) {
            initEncoder()
        }

        // Encode frame with hardware encoder
        val format = MediaFormat.createVideoFormat(codec, width, height)
        format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
        format.setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
        format.setInteger(MediaFormat.KEY_FRAME_RATE, fps)
        format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 2)

        return ByteArray(0) // Placeholder
    }

    private fun initEncoder() {
        // Initialize MediaCodec with H.265 encoder
    }
}
```

### 5. Input Handler

```kotlin
// input/InputHandler.kt
package com.bonsai.agent.input

import android.content.Context
import android.view.KeyEvent
import android.view.MotionEvent
import kotlin.math.max
import kotlin.math.min

class InputHandler(private val context: Context) {
    
    suspend fun injectTouch(x: Float, y: Float, action: String) {
        // Use AccessibilityService or shell command to inject touch
        val motionAction = when (action) {
            "down" -> MotionEvent.ACTION_DOWN
            "move" -> MotionEvent.ACTION_MOVE
            "up" -> MotionEvent.ACTION_UP
            else -> return
        }

        // Implementation depends on device permissions
        // May require AccessibilityService or adb shell input
    }

    suspend fun injectKey(keyCode: Int, pressed: Boolean) {
        val action = if (pressed) KeyEvent.ACTION_DOWN else KeyEvent.ACTION_UP
        // Inject key event
    }

    suspend fun injectText(text: String) {
        // Type each character
        for (char in text) {
            injectCharacter(char)
        }
    }

    private suspend fun injectCharacter(char: Char) {
        // Convert character to key code and inject
    }
}
```

### 6. File Synchronization

```kotlin
// file/FileSyncService.kt
package com.bonsai.agent.file

import android.content.Context
import android.os.Environment
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import java.io.File
import java.security.MessageDigest

class FileSyncService(private val context: Context) {
    private val syncRoot = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "bonsai_sync")

    suspend fun sync(direction: String) = withContext(Dispatchers.IO) {
        when (direction) {
            "push" -> pushFiles()
            "pull" -> pullFiles()
            "bidirectional" -> {
                pushFiles()
                pullFiles()
            }
        }
    }

    private suspend fun pushFiles() {
        // Scan local files and send to desktop
        scanDirectory(syncRoot)
    }

    private suspend fun pullFiles() {
        // Receive files from desktop and write to local storage
    }

    private fun scanDirectory(dir: File): List<FileMetadata> {
        return dir.walkTopDown()
            .filter { it.isFile }
            .map { file ->
                FileMetadata(
                    path = file.relativeTo(syncRoot).path,
                    size = file.length(),
                    lastModified = file.lastModified(),
                    hash = calculateHash(file)
                )
            }
            .toList()
    }

    private fun calculateHash(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val buffer = ByteArray(4096)
        file.inputStream().use { stream ->
            var bytesRead = stream.read(buffer)
            while (bytesRead > 0) {
                digest.update(buffer, 0, bytesRead)
                bytesRead = stream.read(buffer)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
}

data class FileMetadata(
    val path: String,
    val size: Long,
    val lastModified: Long,
    val hash: String
)
```

### 7. Capability Checking

```kotlin
// capability/CapabilityChecker.kt
package com.bonsai.agent.capability

import android.util.Base64
import java.security.Signature
import java.security.PublicKey
import java.time.Instant

class CapabilityChecker {
    private val publicKey: PublicKey? = null // Would be loaded from device identity

    fun verify(tokenString: String, capability: String? = null): Boolean {
        return try {
            val token = CapabilityToken.parse(tokenString)
            
            // Check expiration
            if (token.expiresAt < Instant.now().epochSecond) {
                return false
            }

            // Check revocation
            if (token.revoked) {
                return false
            }

            // Verify signature
            publicKey?.let {
                val sig = Signature.getInstance("Ed25519")
                sig.initVerify(it)
                sig.update(token.payload.toByteArray())
                return sig.verify(Base64.decode(token.signature, Base64.DEFAULT))
            } ?: true
        } catch (e: Exception) {
            false
        }
    }
}

data class CapabilityToken(
    val id: String,
    val capability: String,
    val deviceId: String,
    val subject: String,
    val issuedAt: Long,
    val expiresAt: Long,
    val revoked: Boolean,
    val payload: String,
    val signature: String
) {
    companion object {
        fun parse(tokenString: String): CapabilityToken {
            // Parse JWT-style token
            val parts = tokenString.split(".")
            // Implementation...
            error("Not implemented")
        }
    }
}
```

### 8. Telemetry

```kotlin
// telemetry/TelemetryReporter.kt
package com.bonsai.agent.telemetry

import android.util.Log
import kotlinx.coroutines.*
import java.time.Instant

class TelemetryReporter {
    private val tag = "BonsaiAgent"

    fun logInputEvent(type: String) {
        Log.i(tag, "Input event: $type at ${Instant.now()}")
    }

    fun logFileSyncStart(direction: String) {
        Log.i(tag, "File sync started: $direction")
    }

    fun logAppInstallStart(apkUrl: String) {
        Log.i(tag, "App install: $apkUrl")
    }

    fun logCapabilityDenied(capability: String) {
        Log.w(tag, "Capability denied: $capability")
    }

    fun logUnknownMessage(type: String) {
        Log.w(tag, "Unknown message type: $type")
    }
}
```

## Build Configuration

### build.gradle.kts

```kotlin
plugins {
    id("com.android.application")
    kotlin("android")
}

android {
    compileSdk = 33
    namespace = "com.bonsai.agent"

    defaultConfig {
        applicationId = "com.bonsai.agent"
        minSdk = 24
        targetSdk = 33
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = "11"
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("androidx.core:core:1.10.0")
    implementation("androidx.core:core-ktx:1.10.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.0")

    // Cryptography
    implementation("androidx.security:security-crypto:1.1.0-alpha06")

    // Testing
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
}
```

## AndroidManifest.xml

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.bonsai.agent">

    <!-- Required permissions -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.BODY_SENSORS" />
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />

    <application
        android:allowBackup="true"
        android:icon="@mipmap/ic_launcher"
        android:label="@string/app_name"
        android:theme="@style/AppTheme">

        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>

        <service android:name=".BonsaiService" />
        <service
            android:name=".accessibility.BonsaiAccessibilityService"
            android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE">
            <intent-filter>
                <action android:name="android.accessibilityservice.AccessibilityService" />
            </intent-filter>
        </service>
    </application>
</manifest>
```

## Deployment & Testing

### Build APK
```bash
cd android-agent
./gradlew assembleRelease
```

### Install on Device
```bash
adb install -r app/build/outputs/apk/release/app-release.apk
```

### Grant Permissions
```bash
adb shell pm grant com.bonsai.agent android.permission.BIND_ACCESSIBILITY_SERVICE
```

### Start Service
```bash
adb shell am startservice com.bonsai.agent/.BonsaiService
```

### Monitor Logs
```bash
adb logcat | grep BonsaiAgent
```

## Future Enhancements

- [ ] WebRTC for P2P streaming
- [ ] Bluetooth fallback connectivity
- [ ] GPU video encoding (NVENC equivalent)
- [ ] Sensor data streaming
- [ ] App hot-reload support
- [ ] Automated crash reporting
- [ ] Performance profiling tools
