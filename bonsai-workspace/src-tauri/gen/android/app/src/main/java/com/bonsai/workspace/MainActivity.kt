package com.bonsai.workspace

import android.content.Intent
import android.os.Bundle
import android.util.Log

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    if (!supportsRequiredWebView()) {
      Log.e("Bonsai", "Unsupported WebView provider on this device; redirecting to fallback activity")
      startActivity(Intent(this, UnsupportedWebViewActivity::class.java))
      finish()
    }
  }

  private fun supportsRequiredWebView(): Boolean {
    return try {
      // Fire OS builds can miss this class expected by Chromium/Tauri.
      Class.forName("android.webkit.PacProcessor")
      true
    } catch (t: Throwable) {
      Log.e("Bonsai", "WebView compatibility check failed", t)
      false
    }
  }
}
