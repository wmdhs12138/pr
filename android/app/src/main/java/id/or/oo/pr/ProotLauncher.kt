package id.or.oo.pr

import android.util.Log
import java.io.File

class ProotLauncher(private val app: App) {

    companion object {
        private const val TAG = "PR"
    }

    val prefixDir: File
        get() = app.prefixDir

    fun startSession(
        distroName: String,
        user: String = "root",
        isolated: Boolean = false,
        rows: Int = 24,
        cols: Int = 80
    ): Session? {
        val binDir = File(prefixDir, "bin")
        val bash = File(binDir, "bash")
        val script = File(prefixDir, "scripts/proot-distro.sh")

        if (!bash.exists()) {
            Log.e(TAG, "bash not found at $bash")
            return null
        }
        if (!script.exists()) {
            Log.e(TAG, "proot-distro.sh not found at $script")
            return null
        }

        val envVars = buildEnvVars()
        val cmd = bash.absolutePath
        val args = buildArgs(script.absolutePath, distroName, user, isolated)

        val masterFd = PtyNative.forkPty(cmd, args, envVars, rows, cols)
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
        val binDir = File(prefixDir, "bin")
        val bash = File(binDir, "bash")
        val script = File(prefixDir, "scripts/proot-distro.sh")

        if (!bash.exists()) {
            Log.e(TAG, "bash not found at $bash")
            return null
        }

        val envVars = buildEnvVars()
        val fullCommand = "source ${script.absolutePath} 2>/dev/null; $command"
        val args = arrayOf("-c", fullCommand)

        val masterFd = PtyNative.forkPty(bash.absolutePath, args, envVars, rows, cols)
        if (masterFd < 0) {
            Log.e(TAG, "forkPty failed for command: $command")
            return null
        }

        return Session(masterFd)
    }

    private fun buildArgs(scriptPath: String, distroName: String, user: String, isolated: Boolean): Array<String> {
        val args = mutableListOf(scriptPath, "login", distroName, "--user", user)
        if (isolated) {
            args.add("--isolated")
        }
        return args.toTypedArray()
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
