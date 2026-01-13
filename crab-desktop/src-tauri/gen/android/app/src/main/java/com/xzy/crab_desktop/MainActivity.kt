package com.xzy.crab_desktop

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
import android.view.View
import android.view.WindowManager
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    
    // 保持屏幕常亮
    window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    
    // 隐藏系统栏 (沉浸模式)
    hideSystemUI()

    // 启动保活前台服务
    val serviceIntent = Intent(this, KeepAliveService::class.java)
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
        startForegroundService(serviceIntent)
    } else {
        startService(serviceIntent)
    }

    // 请求忽略电池优化
    requestIgnoreBatteryOptimizations()
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) {
      hideSystemUI()
      // 尝试进入屏幕固定模式 (Kiosk Mode)
      // 注意: 非 Device Owner 应用会弹窗请求用户确认
      try {
        startLockTask()
      } catch (e: Exception) {
        // 某些设备或状态下可能会失败，忽略错误以防崩溃
        e.printStackTrace()
      }
    }
  }

  // 拦截返回键
  override fun onBackPressed() {
      // 这里的 super.onBackPressed() 被故意移除，从而屏蔽物理返回键
      // 用户只能通过 App 内部的 Exit 按钮退出
  }

  private fun hideSystemUI() {
    window.decorView.systemUiVisibility = (View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
        or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
        or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
        or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
        or View.SYSTEM_UI_FLAG_FULLSCREEN)
  }

  private fun requestIgnoreBatteryOptimizations() {
      try {
          val intent = Intent()
          val packageName = packageName
          val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
          if (!pm.isIgnoringBatteryOptimizations(packageName)) {
              intent.action = Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS
              intent.data = Uri.parse("package:$packageName")
              startActivity(intent)
          }
      } catch (e: Exception) {
          e.printStackTrace()
      }
  }
}
