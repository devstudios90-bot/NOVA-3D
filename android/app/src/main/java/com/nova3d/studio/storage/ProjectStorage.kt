package com.nova3d.studio.storage

import android.content.ContentResolver
import android.content.Context
import android.graphics.Bitmap
import android.net.Uri
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileOutputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.util.Locale

data class ProjectSummary(
    val fileName: String,
    val title: String,
    val modifiedAtMillis: Long,
    val sizeBytes: Long,
    val nodeCount: Int,
    val thumbnailPath: String?,
)

data class RecoveryInfo(
    val title: String,
    val linkedFileName: String?,
    val updatedAtMillis: Long,
    val sizeBytes: Long,
)

class ProjectStorage(context: Context) {
    private val appContext = context.applicationContext
    private val projectsDir = File(appContext.filesDir, "projects").apply { mkdirs() }
    private val recoveryDir = File(appContext.filesDir, "recovery").apply { mkdirs() }
    private val autosaveFile = File(recoveryDir, "autosave.nova3d")
    private val autosaveMetaFile = File(recoveryDir, "autosave.meta.json")
    private val sessionLockFile = File(recoveryDir, "session.lock")

    suspend fun listProjects(): List<ProjectSummary> = withContext(Dispatchers.IO) {
        val files = projectsDir.listFiles { file ->
            file.isFile && file.name.endsWith(".nova3d")
        } ?: return@withContext emptyList()

        files.map { file ->
            val metaFile = metaFileFor(file.name)
            val thumbFile = thumbFileFor(file.name)
            var title = file.name.removeSuffix(".nova3d").replace('_', ' ')
            var nodeCount = 0
            var modifiedAt = file.lastModified()
            if (metaFile.isFile) {
                runCatching {
                    val json = JSONObject(metaFile.readText(Charsets.UTF_8))
                    title = json.optString("title", title).ifBlank { title }
                    nodeCount = json.optInt("nodeCount", 0)
                    modifiedAt = json.optLong("modifiedAtMillis", modifiedAt)
                }
            }
            ProjectSummary(
                fileName = file.name,
                title = title,
                modifiedAtMillis = modifiedAt,
                sizeBytes = file.length(),
                nodeCount = nodeCount,
                thumbnailPath = thumbFile.takeIf { it.isFile && it.length() > 0L }?.absolutePath,
            )
        }.sortedByDescending { it.modifiedAtMillis }
    }

    suspend fun saveProject(
        title: String,
        existingFileName: String?,
        payload: ByteArray,
        nodeCount: Int,
        thumbnail: Bitmap?,
    ): ProjectSummary = withContext(Dispatchers.IO) {
        val cleanTitle = title.trim().ifBlank { "Untitled Scene" }
        val fileName = existingFileName?.takeIf { it.endsWith(".nova3d") }
            ?: uniqueFileNameForTitle(cleanTitle)
        val target = File(projectsDir, fileName)
        atomicWriteBytes(target, payload)

        val now = System.currentTimeMillis()
        val meta = JSONObject()
            .put("title", cleanTitle)
            .put("nodeCount", nodeCount)
            .put("modifiedAtMillis", now)
        atomicWriteBytes(metaFileFor(fileName), meta.toString().toByteArray(Charsets.UTF_8))

        val thumbFile = thumbFileFor(fileName)
        if (thumbnail != null && !thumbnail.isRecycled) {
            val baos = ByteArrayOutputStream()
            thumbnail.compress(Bitmap.CompressFormat.PNG, 90, baos)
            atomicWriteBytes(thumbFile, baos.toByteArray())
        }

        ProjectSummary(
            fileName = fileName,
            title = cleanTitle,
            modifiedAtMillis = now,
            sizeBytes = target.length(),
            nodeCount = nodeCount,
            thumbnailPath = thumbFile.takeIf { it.isFile && it.length() > 0L }?.absolutePath,
        )
    }

    suspend fun loadProjectBytes(fileName: String): ByteArray = withContext(Dispatchers.IO) {
        val target = File(projectsDir, fileName)
        if (!target.isFile) {
            error("Project file '$fileName' not found.")
        }
        target.readBytes()
    }

    suspend fun deleteProject(fileName: String): Boolean = withContext(Dispatchers.IO) {
        val deletedMain = File(projectsDir, fileName).delete()
        metaFileFor(fileName).delete()
        thumbFileFor(fileName).delete()
        deletedMain
    }

    suspend fun readUriBytes(resolver: ContentResolver, uri: Uri): Pair<String, ByteArray> =
        withContext(Dispatchers.IO) {
            val name = uri.lastPathSegment?.substringAfterLast('/') ?: "imported.nova3d"
            val bytes = resolver.openInputStream(uri)?.use { it.readBytes() }
                ?: error("Unable to open selected .nova3d file for reading.")
            if (bytes.isEmpty()) {
                error("Selected .nova3d file is empty.")
            }
            name to bytes
        }

    suspend fun writeUriBytes(resolver: ContentResolver, uri: Uri, payload: ByteArray) =
        withContext(Dispatchers.IO) {
            resolver.openOutputStream(uri, "w")?.use { out ->
                out.write(payload)
                out.flush()
            } ?: error("Unable to open destination file for writing.")
        }

    suspend fun setSessionLock(active: Boolean) = withContext(Dispatchers.IO) {
        if (active) {
            atomicWriteBytes(sessionLockFile, System.currentTimeMillis().toString().toByteArray())
        } else {
            sessionLockFile.delete()
        }
    }

    suspend fun writeAutosave(
        title: String,
        linkedFileName: String?,
        dirty: Boolean,
        payload: ByteArray,
    ) = withContext(Dispatchers.IO) {
        if (payload.isEmpty()) return@withContext
        atomicWriteBytes(autosaveFile, payload)
        val now = System.currentTimeMillis()
        val meta = JSONObject()
            .put("title", title.ifBlank { "Untitled Scene" })
            .put("linkedFileName", linkedFileName ?: "")
            .put("dirty", dirty)
            .put("updatedAtMillis", now)
        atomicWriteBytes(autosaveMetaFile, meta.toString().toByteArray(Charsets.UTF_8))
    }

    suspend fun getPendingRecovery(): RecoveryInfo? = withContext(Dispatchers.IO) {
        if (!autosaveFile.isFile || autosaveFile.length() == 0L || !autosaveMetaFile.isFile) {
            return@withContext null
        }
        runCatching {
            val meta = JSONObject(autosaveMetaFile.readText(Charsets.UTF_8))
            val dirty = meta.optBoolean("dirty", false)
            val crashedLock = sessionLockFile.isFile
            if (!dirty && !crashedLock) {
                return@withContext null
            }
            RecoveryInfo(
                title = meta.optString("title", "Recovered Scene").ifBlank { "Recovered Scene" },
                linkedFileName = meta.optString("linkedFileName", "").ifBlank { null },
                updatedAtMillis = meta.optLong("updatedAtMillis", autosaveFile.lastModified()),
                sizeBytes = autosaveFile.length(),
            )
        }.getOrNull()
    }

    suspend fun loadRecoveryBytes(): ByteArray = withContext(Dispatchers.IO) {
        if (!autosaveFile.isFile) {
            error("Autosave file no longer exists.")
        }
        autosaveFile.readBytes()
    }

    suspend fun clearAutosave() = withContext(Dispatchers.IO) {
        autosaveFile.delete()
        autosaveMetaFile.delete()
        sessionLockFile.delete()
    }

    private fun metaFileFor(fileName: String): File =
        File(projectsDir, fileName.removeSuffix(".nova3d") + ".meta.json")

    private fun thumbFileFor(fileName: String): File =
        File(projectsDir, fileName.removeSuffix(".nova3d") + ".png")

    private fun uniqueFileNameForTitle(title: String): String {
        val base = title.lowercase(Locale.US)
            .replace(Regex("[^a-z0-9]+"), "_")
            .trim('_')
            .ifBlank { "scene" }
            .take(40)
        var candidate = "$base.nova3d"
        var index = 2
        while (File(projectsDir, candidate).exists()) {
            candidate = "${base}_$index.nova3d"
            index++
        }
        return candidate
    }

    private fun atomicWriteBytes(target: File, bytes: ByteArray) {
        val parent = target.parentFile ?: appContext.filesDir
        if (!parent.exists()) {
            parent.mkdirs()
        }
        val tmp = File(parent, ".${target.name}.${System.nanoTime()}.tmp")
        try {
            FileOutputStream(tmp).use { fos ->
                fos.write(bytes)
                fos.flush()
                runCatching { fos.fd.sync() }
            }
            try {
                Files.move(
                    tmp.toPath(),
                    target.toPath(),
                    StandardCopyOption.REPLACE_EXISTING,
                    StandardCopyOption.ATOMIC_MOVE,
                )
            } catch (_: Throwable) {
                if (target.exists()) {
                    target.delete()
                }
                if (!tmp.renameTo(target)) {
                    tmp.copyTo(target, overwrite = true)
                    tmp.delete()
                }
            }
        } catch (t: Throwable) {
            tmp.delete()
            throw t
        }
    }
}
