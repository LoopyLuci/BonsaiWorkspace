package com.bonsai.workspace

import android.graphics.BitmapFactory
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.widget.Button
import android.widget.EditText
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

class RemoteSurfaceActivity : AppCompatActivity() {
  private val ui = Handler(Looper.getMainLooper())
  private val worker = Executors.newSingleThreadExecutor()
  private val running = AtomicBoolean(false)
  private val stoppingSession = AtomicBoolean(false)

  private lateinit var hostInput: EditText
  private lateinit var portInput: EditText
  private lateinit var tokenInput: EditText
  private lateinit var statusView: TextView
  private lateinit var frameView: ImageView
  private lateinit var connectButton: Button
  private lateinit var disconnectButton: Button
  private lateinit var sendTextInput: EditText
  private lateinit var sendTextButton: Button

  private var sessionId: String = ""

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    title = "Bonsai Buddy"

    val host = intent?.getStringExtra("desktop_host") ?: "127.0.0.1"
    val port = intent?.getIntExtra("desktop_port", 11369) ?: 11369
    val token = intent?.getStringExtra("pair_token") ?: ""
    sessionId = intent?.getStringExtra("session_id") ?: ""

    val rootScroll = ScrollView(this)
    val root = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      gravity = Gravity.CENTER_HORIZONTAL
      setPadding(dp(16), dp(16), dp(16), dp(16))
    }

    val heading = TextView(this).apply {
      text = "Remote Surface Mode"
      textSize = 21f
      gravity = Gravity.CENTER
    }

    val hintText = TextView(this).apply {
      text = "Streams desktop Bonsai UI for Fire/non-WebView devices. Connect over USB reverse (127.0.0.1) or WiFi to desktop host."
      textSize = 14f
      gravity = Gravity.CENTER
      setPadding(0, dp(10), 0, dp(10))
    }

    hostInput = EditText(this).apply {
      setText(host)
      hint = "Desktop host"
    }
    portInput = EditText(this).apply {
      setText(port.toString())
      hint = "Desktop API port"
      inputType = android.text.InputType.TYPE_CLASS_NUMBER
    }
    tokenInput = EditText(this).apply {
      setText(token)
      hint = "Pair token"
    }

    val row = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER
    }

    connectButton = Button(this).apply {
      text = "Connect"
      setOnClickListener { startRemoteLoop() }
    }

    disconnectButton = Button(this).apply {
      text = "Disconnect"
      isEnabled = false
      setOnClickListener { stopRemoteLoop() }
    }

    row.addView(connectButton)
    row.addView(TextView(this).apply { text = "   " })
    row.addView(disconnectButton)

    statusView = TextView(this).apply {
      text = "Disconnected"
      textSize = 13f
      setPadding(0, dp(8), 0, dp(8))
    }

    frameView = ImageView(this).apply {
      adjustViewBounds = true
      minimumHeight = dp(260)
      setBackgroundColor(0xFF101826.toInt())
      setOnTouchListener { _, event ->
        if (!running.get()) return@setOnTouchListener false
        if (event.action == MotionEvent.ACTION_UP) {
          sendTap(event.x.toInt(), event.y.toInt())
        }
        true
      }
    }

    val sendRow = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER_VERTICAL
      setPadding(0, dp(10), 0, 0)
    }

    sendTextInput = EditText(this).apply {
      hint = "Send text to desktop"
      layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
    }

    sendTextButton = Button(this).apply {
      text = "Send Text"
      setOnClickListener {
        val t = sendTextInput.text?.toString()?.trim().orEmpty()
        if (t.isNotEmpty()) {
          sendText(t)
        }
      }
    }

    sendRow.addView(sendTextInput)
    sendRow.addView(TextView(this).apply { text = "  " })
    sendRow.addView(sendTextButton)

    root.addView(heading)
    root.addView(hintText)
    root.addView(labeled("Desktop Host", hostInput))
    root.addView(labeled("Desktop API Port", portInput))
    root.addView(labeled("Pair Token", tokenInput))
    root.addView(row)
    root.addView(statusView)
    root.addView(frameView)
    root.addView(sendRow)

    rootScroll.addView(root)
    setContentView(rootScroll)
  }

  override fun onDestroy() {
    stopRemoteLoop()
    worker.shutdownNow()
    super.onDestroy()
  }

  private fun startRemoteLoop() {
    if (running.get()) return
    running.set(true)
    connectButton.isEnabled = false
    disconnectButton.isEnabled = true
    setStatus("Connecting...")

    worker.execute {
      try {
        ensureSessionStarted()
        setStatus("Connected (session: $sessionId)")

        while (running.get()) {
          fetchFrame()
          try {
            Thread.sleep(350)
          } catch (_: InterruptedException) {
            break
          }
        }
      } catch (t: Throwable) {
        Log.e("Bonsai", "Remote surface loop failed", t)
        setStatus("Error: ${t.message}")
      } finally {
        running.set(false)
        ui.post {
          connectButton.isEnabled = true
          disconnectButton.isEnabled = false
        }
      }
    }
  }

  private fun stopRemoteLoop() {
    running.set(false)
    connectButton.isEnabled = true
    disconnectButton.isEnabled = false
    setStatus("Disconnected")

    if (sessionId.isNotBlank() && stoppingSession.compareAndSet(false, true)) {
      Thread {
        try {
          stopRemoteSession()
        } catch (t: Throwable) {
          Log.w("Bonsai", "Remote session stop failed", t)
        } finally {
          stoppingSession.set(false)
        }
      }.start()
    }
  }

  private fun stopRemoteSession() {
    val query = "session_id=${urlEncode(sessionId)}"
    val conn = openConnection(
      endpoint = "/remote/surface/session/stop",
      query = query,
      method = "POST",
      body = null,
    )

    try {
      if (conn.responseCode !in 200..299) {
        Log.w("Bonsai", "session/stop returned HTTP ${conn.responseCode}: ${readText(conn)}")
      }
    } finally {
      conn.disconnect()
    }
  }

  private fun ensureSessionStarted() {
    if (sessionId.isNotBlank()) {
      return
    }

    val token = tokenInput.text?.toString()?.trim().orEmpty()
    val conn = openConnection(
      endpoint = "/remote/surface/session/start",
      query = null,
      method = "POST",
      body = JSONObject().apply {
        if (token.isNotEmpty()) {
          put("token", token)
        }
      }.toString(),
    )

    try {
      val code = conn.responseCode
      val body = readText(conn)
      if (code !in 200..299) {
        throw IllegalStateException("session/start failed: HTTP $code $body")
      }
      val json = JSONObject(body)
      sessionId = json.optString("session_id", "")
      if (sessionId.isBlank()) {
        throw IllegalStateException("session/start missing session_id")
      }
    } finally {
      conn.disconnect()
    }
  }

  private fun fetchFrame() {
    val token = tokenInput.text?.toString()?.trim().orEmpty()
    val query = buildString {
      append("session_id=")
      append(urlEncode(sessionId))
      if (token.isNotEmpty()) {
        append("&token=")
        append(urlEncode(token))
      }
    }

    val conn = openConnection(
      endpoint = "/remote/surface/frame",
      query = query,
      method = "GET",
      body = null,
    )

    try {
      val code = conn.responseCode
      if (code !in 200..299) {
        val err = readText(conn)
        throw IllegalStateException("frame failed: HTTP $code $err")
      }
      val bytes = conn.inputStream.readBytes()
      val bmp = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
      if (bmp != null) {
        ui.post { frameView.setImageBitmap(bmp) }
      }
    } finally {
      conn.disconnect()
    }
  }

  private fun sendTap(x: Int, y: Int) {
    sendInput(
      JSONObject().apply {
        put("event_type", "click")
        put("x", x)
        put("y", y)
        put("button", "left")
      }.toString(),
    )
  }

  private fun sendText(text: String) {
    sendInput(
      JSONObject().apply {
        put("event_type", "text_input")
        put("text", text)
      }.toString(),
    )
  }

  private fun sendInput(payload: String) {
    if (!running.get()) return

    worker.execute {
      try {
        val token = tokenInput.text?.toString()?.trim().orEmpty()
        val query = buildString {
          append("session_id=")
          append(urlEncode(sessionId))
          if (token.isNotEmpty()) {
            append("&token=")
            append(urlEncode(token))
          }
        }

        val conn = openConnection(
          endpoint = "/remote/surface/input",
          query = query,
          method = "POST",
          body = payload,
        )

        try {
          if (conn.responseCode !in 200..299) {
            Log.e("Bonsai", "input failed: ${conn.responseCode} ${readText(conn)}")
          }
        } finally {
          conn.disconnect()
        }
      } catch (t: Throwable) {
        Log.e("Bonsai", "sendInput failed", t)
      }
    }
  }

  private fun baseUrl(): String {
    val host = hostInput.text?.toString()?.trim().orEmpty().ifEmpty { "127.0.0.1" }
    val port = portInput.text?.toString()?.trim().orEmpty().ifEmpty { "11369" }
    return "http://$host:$port"
  }

  private fun openConnection(
    endpoint: String,
    query: String?,
    method: String,
    body: String?,
  ): HttpURLConnection {
    val url = if (!query.isNullOrBlank()) {
      URL("${baseUrl()}$endpoint?$query")
    } else {
      URL("${baseUrl()}$endpoint")
    }

    val conn = (url.openConnection() as HttpURLConnection).apply {
      requestMethod = method
      connectTimeout = 5000
      readTimeout = 10000
      setRequestProperty("Accept", "application/json")
      if (!body.isNullOrBlank()) {
        doOutput = true
        setRequestProperty("Content-Type", "application/json")
        outputStream.use { os ->
          os.write(body.toByteArray(Charsets.UTF_8))
        }
      }
    }

    val token = tokenInput.text?.toString()?.trim().orEmpty()
    if (token.isNotEmpty()) {
      conn.setRequestProperty("x-bonsai-token", token)
    }

    return conn
  }

  private fun readText(conn: HttpURLConnection): String {
    val stream = try {
      conn.inputStream
    } catch (_: Throwable) {
      conn.errorStream
    } ?: return ""

    return stream.use {
      BufferedReader(InputStreamReader(it)).readText()
    }
  }

  private fun labeled(label: String, view: EditText): LinearLayout {
    return LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setPadding(0, dp(6), 0, dp(6))
      addView(TextView(this@RemoteSurfaceActivity).apply {
        text = label
        textSize = 12f
      })
      addView(view)
    }
  }

  private fun setStatus(message: String) {
    ui.post { statusView.text = message }
  }

  private fun dp(value: Int): Int {
    return (value * resources.displayMetrics.density).toInt()
  }

  private fun urlEncode(value: String): String {
    return java.net.URLEncoder.encode(value, "UTF-8")
  }
}
