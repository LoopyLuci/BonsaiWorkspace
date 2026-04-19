package com.bonsai.workspace

import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

class UnsupportedWebViewActivity : AppCompatActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setTitle("Bonsai Buddy")

    val outer = ScrollView(this)
    val container = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      gravity = Gravity.CENTER_HORIZONTAL
      setPadding(dp(24), dp(40), dp(24), dp(28))
    }

    val title = TextView(this).apply {
      text = "Bonsai Buddy"
      textSize = 24f
      gravity = Gravity.CENTER
    }

    val heading = TextView(this).apply {
      text = "Runtime mode required on this Fire OS build"
      textSize = 18f
      gravity = Gravity.CENTER
      setPadding(0, dp(18), 0, dp(10))
    }

    val body = TextView(this).apply {
      text = "This device is missing required Android WebView components, so the built-in UI mode cannot start here.\n\n" +
        "You can continue by using Remote Surface mode (recommended), which streams the same desktop Bonsai experience."
      textSize = 15f
      gravity = Gravity.CENTER
      setLineSpacing(0f, 1.2f)
    }

    val row = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER
      setPadding(0, dp(20), 0, 0)
    }

    val remoteButton = Button(this).apply {
      text = "Start Remote Surface"
      setOnClickListener {
        startActivity(Intent(this@UnsupportedWebViewActivity, RemoteSurfaceEntryActivity::class.java).apply {
          putExtra("desktop_host", "127.0.0.1")
          putExtra("desktop_port", 11369)
        })
      }
    }

    val retryButton = Button(this).apply {
      text = "Retry"
      setOnClickListener {
        startActivity(Intent(this@UnsupportedWebViewActivity, MainActivity::class.java))
        finish()
      }
    }

    val settingsButton = Button(this).apply {
      text = "Open App Settings"
      setOnClickListener {
        startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
          data = android.net.Uri.fromParts("package", packageName, null)
        })
      }
    }

    row.addView(remoteButton)
    row.addView(TextView(this).apply { text = "   " })
    row.addView(retryButton)
    row.addView(TextView(this).apply { text = "   " })
    row.addView(settingsButton)

    container.addView(title)
    container.addView(heading)
    container.addView(body)
    container.addView(row)
    outer.addView(container)
    setContentView(outer)
  }

  private fun dp(value: Int): Int {
    return (value * resources.displayMetrics.density).toInt()
  }
}
