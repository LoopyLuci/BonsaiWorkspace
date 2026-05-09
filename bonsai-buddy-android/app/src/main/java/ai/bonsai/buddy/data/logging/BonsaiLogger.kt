package ai.bonsai.buddy.data.logging

import android.content.Context
import android.util.Log
import androidx.core.content.FileProvider
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.concurrent.atomic.AtomicInteger
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class BonsaiLogger @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private val counter = AtomicInteger(0)
    private val formatter = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US)
    private val logsDir: File by lazy {
        File(context.filesDir, "logs").apply { mkdirs() }
    }
    private val logFile: File by lazy { File(logsDir, "bonsai-mobile.log") }

    fun d(tag: String, message: String) = write("D", tag, message)
    fun i(tag: String, message: String) = write("I", tag, message)
    fun w(tag: String, message: String) = write("W", tag, message)
    fun e(tag: String, message: String, tr: Throwable? = null) =
        write("E", tag, "$message${tr?.let { " | ${it.message}" } ?: ""}")

    fun getLogFile(): File = logFile

    fun shareLogAuthority(): String = "${context.packageName}.fileprovider"

    private fun write(level: String, tag: String, message: String) {
        val line = "${formatter.format(Date())} $level/$tag [${counter.incrementAndGet()}] $message"
        when (level) {
            "D" -> Log.d(tag, message)
            "I" -> Log.i(tag, message)
            "W" -> Log.w(tag, message)
            else -> Log.e(tag, message)
        }
        runCatching {
            logFile.appendText(line + "\n")
        }
    }
}
