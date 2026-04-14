package id.or.oo.pr

import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.OnBackPressedCallback
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.sp
import org.connectbot.terminal.TerminalEmulator
import org.connectbot.terminal.TerminalEmulatorFactory
import org.connectbot.terminal.Terminal
import java.io.File
import kotlin.concurrent.thread

class TerminalActivity : ComponentActivity() {

    companion object {
        private const val TAG = "PR"
    }

    private var session: ProotLauncher.Session? = null
    private var emulator: TerminalEmulator? = null
    private var readerThread: Thread? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val distroName = intent.getStringExtra("distro") ?: run {
            finish()
            return
        }

        val app = application as App
        val launcher = ProotLauncher(app)

        val em = TerminalEmulatorFactory.create(
            initialRows = 24,
            initialCols = 80,
            defaultForeground = Color.White,
            defaultBackground = Color(0xFF1a1a2e),
            onKeyboardInput = { data ->
                session?.write(data)
            }
        )
        emulator = em

        val sess = launcher.startSession(distroName)
        if (sess == null) {
            Log.e(TAG, "Failed to start session for $distroName")
            finish()
            return
        }
        session = sess

        readerThread = thread(name = "pty-reader") {
            val buf = ByteArray(8192)
            while (!sess.closed) {
                try {
                    val n = sess.read(buf)
                    if (n < 0) break
                    if (n > 0) {
                        em.writeInput(buf, 0, n)
                    }
                } catch (e: Exception) {
                    if (!sess.closed) Log.e(TAG, "PTY read error", e)
                    break
                }
            }
            Log.i(TAG, "PTY reader thread exited")
        }

        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                cleanup()
                finish()
            }
        })

        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = Color(0xFF1a1a2e)
                ) {
                    Terminal(
                        terminalEmulator = em,
                        modifier = Modifier.fillMaxSize(),
                        initialFontSize = 12.sp,
                        backgroundColor = Color(0xFF1a1a2e),
                        foregroundColor = Color.White,
                        keyboardEnabled = true,
                        showSoftKeyboard = true,
                    )
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        cleanup()
    }

    private fun cleanup() {
        session?.close()
        session = null
        readerThread?.interrupt()
        readerThread = null
    }
}
