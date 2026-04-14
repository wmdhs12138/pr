package id.or.oo.pr

import android.app.Application
import android.content.Context
import android.content.SharedPreferences
import android.os.Build
import android.util.Log
import java.io.File
import java.io.FileOutputStream
import java.io.IOException

class App : Application() {

    companion object {
        private const val TAG = "PR"
        private const val PREFS_NAME = "pr_prefs"
        private const val KEY_INITIALIZED = "bootstrapped"
        private const val KEY_VERSION = "bootstrap_version"

        private const val BOOTSTRAP_VERSION = 1
    }

    val prefixDir: File
        get() = File(filesDir, "usr")

    val homeDir: File
        get() = File(filesDir, "home")

    override fun onCreate() {
        super.onCreate()
        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val currentVersion = prefs.getInt(KEY_VERSION, 0)

        if (currentVersion < BOOTSTRAP_VERSION) {
            bootstrap(prefs)
        } else {
            Log.i(TAG, "Already bootstrapped (version $currentVersion)")
        }
    }

    private fun bootstrap(prefs: SharedPreferences) {
        Log.i(TAG, "Running bootstrap...")
        val t0 = System.currentTimeMillis()

        try {
            val binDir = File(prefixDir, "bin")
            val etcDir = File(prefixDir, "etc/proot-distro")
            val scriptsDir = File(prefixDir, "scripts")
            val pluginsDir = File(prefixDir, "plugins")

            binDir.mkdirs()
            etcDir.mkdirs()
            scriptsDir.mkdirs()
            pluginsDir.mkdirs()
            homeDir.mkdirs()
            File(prefixDir, "tmp").mkdirs()

            copyAssetBinary("bin/busybox", File(binDir, "busybox"))
            copyAssetBinary("bin/bash", File(binDir, "bash"))
            copyProotFromNativeLib(binDir)
            copyAssetFile("scripts/bootstrap.sh", File(binDir, "bootstrap.sh"))
            copyAssetFile("scripts/proot-distro.sh", File(scriptsDir, "proot-distro.sh"))
            copyAssetPlugins(pluginsDir, etcDir)

            executeBootstrap()

            prefs.edit()
                .putInt(KEY_VERSION, BOOTSTRAP_VERSION)
                .putBoolean(KEY_INITIALIZED, true)
                .apply()

            val elapsed = System.currentTimeMillis() - t0
            Log.i(TAG, "Bootstrap complete (${elapsed}ms)")
        } catch (e: Exception) {
            Log.e(TAG, "Bootstrap failed", e)
        }
    }

    private fun copyAssetBinary(assetPath: String, dest: File) {
        assets.open(assetPath).use { input ->
            FileOutputStream(dest).use { output ->
                input.copyTo(output)
            }
        }
        dest.setExecutable(true, false)
        dest.setReadable(true, false)
        Log.d(TAG, "Installed: ${dest.name} (${dest.length()} bytes)")
    }

    private fun copyProotFromNativeLib(binDir: File) {
        val nativeDir = File(applicationInfo.nativeLibraryDir)
        val libProot = File(nativeDir, "libproot.so")
        val dest = File(binDir, "proot")

        if (libProot.exists()) {
            libProot.copyTo(dest, overwrite = true)
            dest.setExecutable(true, false)
            dest.setReadable(true, false)
            Log.d(TAG, "Installed: proot from native lib (${dest.length()} bytes)")
        } else {
            Log.w(TAG, "libproot.so not found at $nativeDir, trying ABI split path")
            val altPath = File(
                applicationInfo.sourceDir.replace(".apk", ""),
                "lib/arm64/libproot.so"
            )
            if (altPath.exists()) {
                altPath.copyTo(dest, overwrite = true)
                dest.setExecutable(true, false)
                dest.setReadable(true, false)
                Log.d(TAG, "Installed: proot from alt path (${dest.length()} bytes)")
            } else {
                Log.e(TAG, "proot binary not found!")
            }
        }
    }

    private fun copyAssetFile(assetPath: String, dest: File) {
        assets.open(assetPath).use { input ->
            FileOutputStream(dest).use { output ->
                input.copyTo(output)
            }
        }
        dest.setExecutable(true, false)
        dest.setReadable(true, false)
        Log.d(TAG, "Installed: ${dest.name}")
    }

    private fun copyAssetPlugins(pluginsStaging: File, etcDir: File) {
        val pluginFiles = assets.list("plugins") ?: emptyArray()
        var count = 0
        for (name in pluginFiles) {
            if (name.endsWith(".sh")) {
                val dest = File(etcDir, name)
                assets.open("plugins/$name").use { input ->
                    FileOutputStream(dest).use { output ->
                        input.copyTo(output)
                    }
                }
                dest.setReadable(true, false)
                count++
            }
        }
        Log.d(TAG, "Installed: $count plugins")
    }

    private fun executeBootstrap() {
        val bootstrapScript = File(prefixDir, "bin/bootstrap.sh")
        if (!bootstrapScript.exists()) {
            Log.e(TAG, "bootstrap.sh not found")
            return
        }

        val env = mapOf(
            "APP_PREFIX" to prefixDir.absolutePath,
            "APP_HOME" to homeDir.absolutePath,
            "APP_PACKAGE" to packageName,
            "PATH" to "${File(prefixDir, "bin").absolutePath}",
            "PROOT_NO_SECCOMP" to "1",
            "HOME" to homeDir.absolutePath,
        )

        val cmd = arrayOf(
            "/system/bin/sh",
            bootstrapScript.absolutePath,
        )

        val pb = ProcessBuilder(*cmd)
        pb.environment().putAll(env)
        pb.redirectErrorStream(true)

        Log.d(TAG, "Executing: ${cmd.joinToString(" ")}")
        Log.d(TAG, "APP_PREFIX=${env["APP_PREFIX"]}")

        val proc = pb.start()
        val output = proc.inputStream.bufferedReader().readText()
        val exitCode = proc.waitFor()

        if (output.isNotBlank()) {
            for (line in output.lines()) {
                Log.d(TAG, "[bootstrap] $line")
            }
        }

        if (exitCode != 0) {
            throw IOException("bootstrap.sh failed with exit code $exitCode")
        }
    }
}
