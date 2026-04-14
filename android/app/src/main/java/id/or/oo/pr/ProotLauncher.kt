package id.or.oo.pr

import android.util.Log
import java.io.File

class ProotLauncher(private val app: App) {

    companion object {
        private const val TAG = "PR"
    }

    val prefixDir: File
        get() = app.prefixDir

    private val bashPath: String
        get() = File(app.nativeLibDir, "libbash.so").absolutePath

    fun startSession(
        distroName: String,
        user: String = "root",
        isolated: Boolean = false,
        rows: Int = 24,
        cols: Int = 80
    ): Session? {
        val script = File(prefixDir, "scripts/proot-distro.sh")
        if (!script.exists()) {
            Log.e(TAG, "proot-distro.sh not found at $script")
            return null
        }

        val envVars = buildEnvVars()
        val loginCmd = "${script.absolutePath} login $distroName --user $user"
        val args = arrayOf("sh", "-c", "$bashPath -c '$loginCmd'")

        val masterFd = PtyNative.forkPty("/system/bin/sh", args, envVars, rows, cols)
        if (masterFd < 0) {
            Log.e(TAG, "forkPty failed with fd=$masterFd")
            return null
        }

        Log.i(TAG, "PTY session started for $distroName, masterFd=$masterFd")
        return Session(masterFd)
    }

    fun runCommand(
        command: String,
        rows: Int = 24,
        cols: Int = 80,
    ): Session? {
        val script = File(prefixDir, "scripts/proot-distro.sh")
        val envVars = buildEnvVars()
        val fullCommand = "$bashPath -c 'source ${script.absolutePath} 2>/dev/null; $command'"
        val args = arrayOf("sh", "-c", fullCommand)

        val masterFd = PtyNative.forkPty("/system/bin/sh", args, envVars, rows, cols)
        if (masterFd < 0) {
            Log.e(TAG, "forkPty failed for command: $command")
            return null
        }

        Log.i(TAG, "PTY command started: $command, masterFd=$masterFd")
        return Session(masterFd)
    }

    private fun buildEnvVars(): Array<String> {
        val binDir = File(prefixDir, "bin")
        val homeDir = app.homeDir
        val prefix = prefixDir.absolutePath

        return arrayOf(
            "APP_PREFIX", prefix,
            "APP_HOME", homeDir.absolutePath,
            "APP_PACKAGE", app.packageName,
            "PATH", "${binDir.absolutePath}:/system/bin:/system/xbin",
            "HOME", homeDir.absolutePath,
            "PROOT_NO_SECCOMP", "1",
            "TERM", "xterm-256color",
            "LANG", "en_US.UTF-8",
        )
    }

    class Session(val masterFd: Int) {
        var closed = false
            private set

        fun read(buf: ByteArray, offset: Int = 0, length: Int = buf.size): Int {
            if (closed) return -1
            return PtyNative.read(masterFd, buf, offset, length)
        }

        fun write(data: ByteArray): Int {
            if (closed) return -1
            return PtyNative.write(masterFd, data, 0, data.size)
        }

        fun resize(rows: Int, cols: Int): Int {
            if (closed) return -1
            return PtyNative.resize(masterFd, rows, cols)
        }

        fun close() {
            if (!closed) {
                closed = true
                try {
                    PtyNative.write(masterFd, byteArrayOf(0x04), 0, 1)
                } catch (_: Exception) {}
                try {
                    PtyNative.close(masterFd)
                } catch (_: Exception) {}
            }
        }
    }
}
