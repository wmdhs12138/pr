package id.or.oo.pr

import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Download
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

data class DistroInfo(
    val name: String,
    val displayName: String,
    val isInstalled: Boolean,
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as App
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    DistroListScreen(app)
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DistroListScreen(app: App) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var distros by remember { mutableStateOf<List<DistroInfo>>(emptyList()) }
    var loadingDistro by remember { mutableStateOf<String?>(null) }
    var outputLines by remember { mutableStateOf(mutableListOf<String>()) }
    var showOutput by remember { mutableStateOf(false) }

    fun refreshDistros() {
        val pluginsDir = File(app.prefixDir, "etc/proot-distro")
        val rootfsDir = File(app.prefixDir, "var/lib/proot-distro/installed-rootfs")

        val plugins = pluginsDir.listFiles { f -> f.name.endsWith(".sh") }
            ?.map { f ->
                val distroName = f.nameWithoutExtension
                val displayName = parsePluginName(f) ?: distroName.replaceFirstChar { it.uppercase() }
                val isInstalled = File(rootfsDir, distroName).exists()
                DistroInfo(distroName, displayName, isInstalled)
            }
            ?.sortedBy { it.displayName }
            ?: emptyList()

        distros = plugins
    }

    LaunchedEffect(Unit) { refreshDistros() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("PR") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                )
            )
        }
    ) { padding ->
        if (showOutput) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (loadingDistro != null) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(16.dp),
                                strokeWidth = 2.dp
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                        }
                        Text("Output", style = MaterialTheme.typography.titleMedium)
                    }
                    TextButton(onClick = {
                        showOutput = false
                        outputLines = mutableListOf()
                        refreshDistros()
                    }) {
                        Text("Close")
                    }
                }
                val scrollState = rememberScrollState()
                LaunchedEffect(outputLines.size) {
                    scrollState.animateScrollTo(Int.MAX_VALUE)
                }
                Text(
                    text = outputLines.joinToString("\n"),
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(8.dp)
                        .verticalScroll(scrollState),
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodySmall
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
            ) {
                items(distros) { distro ->
                    DistroRow(
                        distro = distro,
                        isLoading = loadingDistro == distro.name,
                        onInstall = {
                            if (loadingDistro == null) {
                                loadingDistro = distro.name
                                showOutput = true
                                outputLines = mutableListOf("Installing ${distro.displayName}...")
                                scope.launch {
                                    runDistroCommand(app, "install ${distro.name}") { line ->
                                        outputLines = (outputLines + line).toMutableList()
                                    }
                                    loadingDistro = null
                                }
                            }
                        },
                        onLogin = {
                            if (loadingDistro == null) {
                                val intent = Intent(context, TerminalActivity::class.java)
                                intent.putExtra("distro", distro.name)
                                context.startActivity(intent)
                            }
                        },
                        onRemove = {
                            if (loadingDistro == null) {
                                loadingDistro = distro.name
                                showOutput = true
                                outputLines = mutableListOf("Removing ${distro.displayName}...")
                                scope.launch {
                                    runDistroCommand(app, "remove ${distro.name}") { line ->
                                        outputLines = (outputLines + line).toMutableList()
                                    }
                                    loadingDistro = null
                                }
                            }
                        },
                    )
                }
            }
        }
    }
}

@Composable
fun DistroRow(
    distro: DistroInfo,
    isLoading: Boolean,
    onInstall: () -> Unit,
    onLogin: () -> Unit,
    onRemove: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp, vertical = 4.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = distro.displayName,
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    text = if (distro.isInstalled) "Installed" else "Not installed",
                    style = MaterialTheme.typography.bodySmall,
                    color = if (distro.isInstalled)
                        MaterialTheme.colorScheme.primary
                    else
                        MaterialTheme.colorScheme.outline
                )
            }

            if (isLoading) {
                CircularProgressIndicator(
                    modifier = Modifier.size(24.dp),
                    strokeWidth = 2.dp
                )
            } else {
                Row {
                    if (distro.isInstalled) {
                        IconButton(onClick = onLogin) {
                            Icon(Icons.Default.PlayArrow, contentDescription = "Login")
                        }
                        IconButton(onClick = onRemove) {
                            Icon(
                                Icons.Default.Delete,
                                contentDescription = "Remove",
                                tint = MaterialTheme.colorScheme.error
                            )
                        }
                    } else {
                        IconButton(onClick = onInstall) {
                            Icon(Icons.Default.Download, contentDescription = "Install")
                        }
                    }
                }
            }
        }
    }
}

private fun parsePluginName(pluginFile: File): String? {
    try {
        val lines = pluginFile.readLines()
        for (line in lines) {
            val trimmed = line.trim()
            if (trimmed.startsWith("DISTRO_NAME=")) {
                return trimmed.substringAfter("DISTRO_NAME=").trim('"', '\'')
            }
        }
    } catch (_: Exception) {}
    return null
}

private suspend fun runDistroCommand(app: App, command: String, onLine: (String) -> Unit) = withContext(Dispatchers.IO) {
    val binDir = File(app.prefixDir, "bin")
    val script = File(app.prefixDir, "scripts/proot-distro.sh").absolutePath

    val env = mapOf(
        "APP_PREFIX" to app.prefixDir.absolutePath,
        "APP_HOME" to app.homeDir.absolutePath,
        "APP_PACKAGE" to app.packageName,
        "PATH" to "${binDir.absolutePath}:/system/bin:/system/xbin",
        "PROOT_NO_SECCOMP" to "1",
        "HOME" to app.homeDir.absolutePath,
        "TERM" to "xterm-256color",
        "TMPDIR" to app.cacheDir.absolutePath,
    )

    val cmd = arrayOf("/system/bin/sh", script, *command.split(" ").toTypedArray())

    android.util.Log.d("PR", "Running: ${cmd.joinToString(" ")}")
    val pb = ProcessBuilder(*cmd)
    pb.environment().putAll(env)
    pb.redirectErrorStream(true)

    try {
        val proc = pb.start()
        val reader = proc.inputStream.bufferedReader()
        var line: String? = reader.readLine()
        while (line != null) {
            val cleaned = line
                .replace(Regex("\\x1B\\[[0-9;]*[mGKHJ]"), "")
                .replace(Regex("\\x1B\\]\\d+;.*?\\x07"), "")
                .replace(Regex("\r"), "")
                .trim()
            if (cleaned.isNotEmpty()) {
                val finalLine = cleaned
                android.util.Log.d("PR", finalLine)
                withContext(Dispatchers.Main) { onLine(finalLine) }
            }
            line = reader.readLine()
        }
        val exitCode = proc.waitFor()
        android.util.Log.d("PR", "Exit code: $exitCode")
        withContext(Dispatchers.Main) {
            if (exitCode != 0) {
                onLine("Exit code: $exitCode")
            } else {
                onLine("Done.")
            }
        }
    } catch (e: Exception) {
        withContext(Dispatchers.Main) { onLine("ERROR: ${e.message}") }
    }
}
