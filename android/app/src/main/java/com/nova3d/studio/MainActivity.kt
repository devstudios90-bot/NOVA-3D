package com.nova3d.studio

import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.nova3d.studio.storage.ProjectStorage
import com.nova3d.studio.ui.NovaAppShell
import com.nova3d.studio.ui.theme.NovaTheme
import com.nova3d.studio.viewport.ViewportController

class MainActivity : ComponentActivity() {
    private val viewportController = ViewportController()
    private lateinit var projectStorage: ProjectStorage
    private var isLifecycleActive by mutableStateOf(false)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // True Edge-to-Edge and Cutout layout
        WindowCompat.setDecorFitsSystemWindows(window, false)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            window.attributes.layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
        hideSystemBars()

        projectStorage = ProjectStorage(applicationContext)

        setContent {
            NovaTheme {
                NovaAppShell(
                    controller = viewportController,
                    storage = projectStorage,
                    isLifecycleActive = isLifecycleActive,
                    modifier = Modifier.fillMaxSize(),
                )
            }
        }

        // Never block the UI thread: initialize Rust core + Filament + shaders on the dedicated render thread.
        viewportController.startSessionAsync(backendPreference = 0)
    }

    override fun onResume() {
        super.onResume()
        isLifecycleActive = true
        hideSystemBars()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            hideSystemBars()
        }
    }

    private fun hideSystemBars() {
        val insetsController = WindowCompat.getInsetsController(window, window.decorView)
        insetsController.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        insetsController.hide(WindowInsetsCompat.Type.systemBars())
    }

    override fun onPause() {
        isLifecycleActive = false
        super.onPause()
    }

    override fun onDestroy() {
        viewportController.destroy()
        super.onDestroy()
    }
}
