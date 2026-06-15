package com.bonsai.workspace

import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

class RemoteSurfaceEntryActivity : AppCompatActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    val remoteIntent = Intent(this, RemoteSurfaceActivity::class.java).apply {
      putExtra("desktop_host", intent?.getStringExtra("desktop_host") ?: "127.0.0.1")
      putExtra("desktop_port", intent?.getIntExtra("desktop_port", 11369) ?: 11369)
      putExtra("pair_token", intent?.getStringExtra("pair_token") ?: "")
      putExtra("session_id", intent?.getStringExtra("session_id") ?: "")
    }

    startActivity(remoteIntent)
    finish()
  }
}
