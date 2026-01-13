package com.xzy.crab_desktop

import android.app.Application

class CrabApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        // Initialize global crash handler
        CrashHandler.instance.init(this)
    }
}
