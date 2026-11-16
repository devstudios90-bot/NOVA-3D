package com.nova3d.studio.render

import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.media.MediaMuxer
import android.net.Uri
import android.view.Surface
import androidx.core.content.FileProvider
import com.nova3d.studio.nativebridge.NovaNativeBridge
import com.nova3d.studio.viewport.ViewportController
import java.io.File
import java.io.FileOutputStream
import java.nio.ByteBuffer
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

enum class RenderResolutionPreset(
    val label: String,
    val width: Int,
    val height: Int,
) {
    P720("720p HD", 1280, 720),
    P1080("1080p FHD", 1920, 1080),
    P1440("1440p QHD", 2560, 1440),
    P2160("4K UHD", 3840, 2160),
}

enum class RenderQualityPreset(
    val label: String,
    val level: Int,
    val defaultMsaa: Int,
    val defaultSsao: Boolean,
    val defaultBloom: Boolean,
) {
    LOW("Low", 0, 1, false, false),
    MEDIUM("Medium", 1, 2, true, false),
    HIGH("High", 2, 4, true, true),
    ULTRA("Ultra", 3, 4, true, true),
}

enum class RenderImageFormat(
    val label: String,
    val ext: String,
    val mimeType: String,
    val compressFormat: Bitmap.CompressFormat,
    val quality: Int,
) {
    PNG("PNG (Lossless)", "png", "image/png", Bitmap.CompressFormat.PNG, 100),
    JPEG("JPEG (95%)", "jpg", "image/jpeg", Bitmap.CompressFormat.JPEG, 95),
}

enum class RenderToneMapping(val label: String, val code: Int) {
    ACES("ACES Filmic", 0),
    FILMIC("Legacy Filmic", 1),
    PBR_NEUTRAL("Khronos PBR Neutral", 2),
}

data class RenderJobConfig(
    val resolution: RenderResolutionPreset = RenderResolutionPreset.P1080,
    val quality: RenderQualityPreset = RenderQualityPreset.HIGH,
    val aaEnabled: Boolean = true,
    val msaaSamples: Int = 4,
    val ssaoEnabled: Boolean = true,
    val bloomEnabled: Boolean = true,
    val dofEnabled: Boolean = false,
    val dofFocusDistance: Float = 4.2f,
    val dofMaxAperture: Float = 0.015f,
    val dofCocScale: Float = 1.0f,
    val toneMapping: RenderToneMapping = RenderToneMapping.ACES,
    val imageFormat: RenderImageFormat = RenderImageFormat.PNG,
    val cameraNodeId: Long = 0L, // 0 = Viewport Orbit Camera
)

data class DeviceRenderCapabilities(
    val availableRamMb: Long,
    val totalRamMb: Long,
    val isLowMemory: Boolean,
    val avcEncoderName: String?,
    val maxVideoWidth: Int,
    val maxVideoHeight: Int,
    val supports4kVideo: Boolean,
    val supports1440pVideo: Boolean,
    val supports1080pVideo: Boolean,
)

data class PreflightDecision(
    val effectiveWidth: Int,
    val effectiveHeight: Int,
    val wasDownscaled: Boolean,
    val warningMessage: String?,
)

data class SavedRenderItem(
    val fileName: String,
    val file: File,
    val isVideo: Boolean,
    val sizeBytes: Long,
    val modifiedAtMs: Long,
)

object OffscreenRenderEngine {
    private const val RENDERS_DIR = "renders"

    fun rendersDir(context: Context): File {
        val dir = File(context.filesDir, RENDERS_DIR)
        if (!dir.exists()) {
            dir.mkdirs()
        }
        return dir
    }

    fun listSavedRenders(context: Context): List<SavedRenderItem> {
        val dir = rendersDir(context)
        val files = dir.listFiles() ?: return emptyList()
        return files
            .filter {
                it.isFile && !it.name.endsWith(".tmp") && (
                    it.name.endsWith(".png", ignoreCase = true) ||
                        it.name.endsWith(".jpg", ignoreCase = true) ||
                        it.name.endsWith(".mp4", ignoreCase = true)
                    )
            }
            .sortedByDescending { it.lastModified() }
            .map {
                SavedRenderItem(
                    fileName = it.name,
                    file = it,
                    isVideo = it.name.endsWith(".mp4", ignoreCase = true),
                    sizeBytes = it.length(),
                    modifiedAtMs = it.lastModified(),
                )
            }
    }

    fun deleteSavedRender(file: File): Boolean = runCatching { file.delete() }.getOrDefault(false)

    fun shareRender(context: Context, file: File) {
        val authority = "${context.packageName}.fileprovider"
        val uri: Uri = FileProvider.getUriForFile(context, authority, file)
        val mime = when {
            file.name.endsWith(".mp4", ignoreCase = true) -> "video/mp4"
            file.name.endsWith(".jpg", ignoreCase = true) -> "image/jpeg"
            else -> "image/png"
        }
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = mime
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        val chooser = Intent.createChooser(intent, "Share ${file.name}").apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        context.startActivity(chooser)
    }

    suspend fun exportRenderToUri(context: Context, file: File, dstUri: Uri): Result<Unit> =
        withContext(Dispatchers.IO) {
            runCatching {
                context.contentResolver.openOutputStream(dstUri)?.use { out ->
                    file.inputStream().use { inp -> inp.copyTo(out) }
                    out.flush()
                } ?: error("Unable to open destination file stream")
                Unit
            }
        }

    fun probeCapabilities(context: Context): DeviceRenderCapabilities {
        val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
        val mi = ActivityManager.MemoryInfo()
        am?.getMemoryInfo(mi)
        val availMb = (mi.availMem / (1024L * 1024L)).coerceAtLeast(0L)
        val totalMb = (mi.totalMem / (1024L * 1024L)).coerceAtLeast(0L)

        var encoderName: String? = null
        var maxW = 1920
        var maxH = 1080
        var sup4k = false
        var sup1440 = false
        var sup1080 = false

        runCatching {
            val list = MediaCodecList(MediaCodecList.REGULAR_CODECS)
            for (info in list.codecInfos) {
                if (!info.isEncoder) continue
                if (!info.supportedTypes.any { it.equals(MediaFormat.MIMETYPE_VIDEO_AVC, ignoreCase = true) }) {
                    continue
                }
                val caps = info.getCapabilitiesForType(MediaFormat.MIMETYPE_VIDEO_AVC) ?: continue
                val vc = caps.videoCapabilities ?: continue
                val wUpper = vc.supportedWidths.upper
                val hUpper = vc.supportedHeights.upper
                val isSw = info.name.contains("c2.android.", ignoreCase = true) ||
                    info.name.contains("OMX.google.", ignoreCase = true)
                // Prefer hardware encoder if available, otherwise take best match
                if (encoderName == null || !isSw) {
                    encoderName = info.name
                    maxW = wUpper
                    maxH = hUpper
                    sup4k = vc.isSizeSupported(3840, 2160)
                    sup1440 = vc.isSizeSupported(2560, 1440)
                    sup1080 = vc.isSizeSupported(1920, 1080)
                    if (!isSw) break
                }
            }
        }

        return DeviceRenderCapabilities(
            availableRamMb = availMb,
            totalRamMb = totalMb,
            isLowMemory = mi.lowMemory,
            avcEncoderName = encoderName,
            maxVideoWidth = maxW,
            maxVideoHeight = maxH,
            supports4kVideo = sup4k,
            supports1440pVideo = sup1440,
            supports1080pVideo = sup1080,
        )
    }

    /**
     * Pre-flight OOM and hardware encoder capability guard (RN-7).
     * Never allows an unchecked allocation that would crash the app; warns cleanly or downscales.
     */
    fun preflightResolution(
        caps: DeviceRenderCapabilities,
        requested: RenderResolutionPreset,
        forVideo: Boolean,
    ): PreflightDecision {
        var target = requested

        if (caps.isLowMemory && target.width > 1280) {
            return PreflightDecision(
                effectiveWidth = 1280,
                effectiveHeight = 720,
                wasDownscaled = true,
                warningMessage = "Device reported low free RAM (${caps.availableRamMb} MiB); downscaled ${target.label} to 720p to prevent OOM.",
            )
        }

        // Still readback RGBA + ARGB Bitmap + encoder needs ~4x-6x frame buffer RAM
        if (!forVideo) {
            if (target == RenderResolutionPreset.P2160 && caps.availableRamMb < 220L) {
                target = RenderResolutionPreset.P1080
                return PreflightDecision(
                    effectiveWidth = target.width,
                    effectiveHeight = target.height,
                    wasDownscaled = true,
                    warningMessage = "Free RAM (${caps.availableRamMb} MiB) is tight for 4K readback (~220 MiB recommended); downscaled to 1080p.",
                )
            }
            if (target == RenderResolutionPreset.P1440 && caps.availableRamMb < 140L) {
                target = RenderResolutionPreset.P1080
                return PreflightDecision(
                    effectiveWidth = target.width,
                    effectiveHeight = target.height,
                    wasDownscaled = true,
                    warningMessage = "Free RAM (${caps.availableRamMb} MiB) is below safe budget for 1440p; downscaled to 1080p.",
                )
            }
            return PreflightDecision(target.width, target.height, false, null)
        }

        // Video: validate MediaCodec H.264 capabilities
        if (target == RenderResolutionPreset.P2160 && !caps.supports4kVideo) {
            val fallback = if (caps.supports1440pVideo) {
                RenderResolutionPreset.P1440
            } else if (caps.supports1080pVideo) {
                RenderResolutionPreset.P1080
            } else {
                RenderResolutionPreset.P720
            }
            return PreflightDecision(
                effectiveWidth = fallback.width,
                effectiveHeight = fallback.height,
                wasDownscaled = true,
                warningMessage = "Device H.264 encoder (${caps.avcEncoderName ?: "default"}, max ${caps.maxVideoWidth}x${caps.maxVideoHeight}) does not support 4K; downscaled safely to ${fallback.label}.",
            )
        }
        if (target == RenderResolutionPreset.P1440 && !caps.supports1440pVideo) {
            val fallback = if (caps.supports1080pVideo) {
                RenderResolutionPreset.P1080
            } else {
                RenderResolutionPreset.P720
            }
            return PreflightDecision(
                effectiveWidth = fallback.width,
                effectiveHeight = fallback.height,
                wasDownscaled = true,
                warningMessage = "Device H.264 encoder does not support 1440p; downscaled safely to ${fallback.label}.",
            )
        }
        if (target == RenderResolutionPreset.P1080 && !caps.supports1080pVideo) {
            return PreflightDecision(
                effectiveWidth = 1280,
                effectiveHeight = 720,
                wasDownscaled = true,
                warningMessage = "Device H.264 encoder capped below 1080p; downscaled safely to 720p.",
            )
        }

        return PreflightDecision(target.width, target.height, false, null)
    }

    suspend fun renderStillToFile(
        context: Context,
        controller: ViewportController,
        config: RenderJobConfig,
    ): Result<Pair<SavedRenderItem, PreflightDecision>> {
        val caps = probeCapabilities(context)
        val preflight = preflightResolution(caps, config.resolution, forVideo = false)
        val w = preflight.effectiveWidth
        val h = preflight.effectiveHeight

        val rgbaRes = controller.runOnRenderThread { ptr ->
            val prevCam = NovaNativeBridge.nativeGetActiveCameraNode(ptr)
            if (config.cameraNodeId != prevCam) {
                NovaNativeBridge.nativeSetActiveCameraNode(ptr, config.cameraNodeId)
            }
            val pixels = NovaNativeBridge.nativeRenderStillRgba(
                ptr,
                w,
                h,
                config.quality.level,
                config.aaEnabled,
                config.msaaSamples,
                config.ssaoEnabled,
                config.bloomEnabled,
                config.dofEnabled,
                config.dofFocusDistance,
                config.dofMaxAperture,
                config.dofCocScale,
                config.toneMapping.code,
            )
            if (config.cameraNodeId != prevCam) {
                NovaNativeBridge.nativeSetActiveCameraNode(ptr, prevCam)
            }
            pixels
        }

        val rgba = rgbaRes.getOrElse { return Result.failure(it) }
            ?: return Result.failure(
                IllegalStateException(
                    controller.errorMessage ?: "Offscreen Filament render failed at ${w}x${h}",
                ),
            )

        return withContext(Dispatchers.IO) {
            runCatching {
                val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
                try {
                    bmp.copyPixelsFromBuffer(ByteBuffer.wrap(rgba))
                    val dir = rendersDir(context)
                    val stamp = System.currentTimeMillis()
                    val base = sanitizePrefix(controller.projectTitle)
                    val name = "${base}_${w}x${h}_${stamp}.${config.imageFormat.ext}"
                    val dst = File(dir, name)
                    val tmp = File(dir, "$name.tmp")
                    FileOutputStream(tmp).use { fos ->
                        val compressed = bmp.compress(
                            config.imageFormat.compressFormat,
                            config.imageFormat.quality,
                            fos,
                        )
                        fos.flush()
                        if (!compressed) error("Bitmap compression failed")
                    }
                    if (!tmp.renameTo(dst)) {
                        tmp.copyTo(dst, overwrite = true)
                        tmp.delete()
                    }
                    val item = SavedRenderItem(
                        fileName = dst.name,
                        file = dst,
                        isVideo = false,
                        sizeBytes = dst.length(),
                        modifiedAtMs = dst.lastModified(),
                    )
                    Pair(item, preflight)
                } finally {
                    bmp.recycle()
                }
            }
        }
    }

    suspend fun renderVideoToMp4(
        context: Context,
        controller: ViewportController,
        config: RenderJobConfig,
        startFrame: Int,
        endFrame: Int,
        fps: Float,
        isCancelled: () -> Boolean,
        onProgress: (done: Int, total: Int, status: String) -> Unit,
    ): Result<Pair<SavedRenderItem, PreflightDecision>> = withContext(Dispatchers.IO) {
        val caps = probeCapabilities(context)
        val preflight = preflightResolution(caps, config.resolution, forVideo = true)
        val w = preflight.effectiveWidth
        val h = preflight.effectiveHeight
        val safeFps = fps.toInt().coerceIn(12, 60)
        val fStart = minOf(startFrame, endFrame)
        val fEnd = maxOf(startFrame, endFrame)
        val totalFrames = (fEnd - fStart + 1).coerceAtLeast(1)

        val dir = rendersDir(context)
        val stamp = System.currentTimeMillis()
        val base = sanitizePrefix(controller.projectTitle)
        val finalName = "${base}_${w}x${h}_${safeFps}fps_${stamp}.mp4"
        val dst = File(dir, finalName)
        val tmp = File(dir, "$finalName.tmp")
        if (tmp.exists()) tmp.delete()

        var codec: MediaCodec? = null
        var muxer: MediaMuxer? = null
        var inputSurface: Surface? = null
        var muxerStarted = false
        var trackIndex = -1
        var surfaceBound = false
        var savedCamId = 0L
        var savedFrame = 1f

        try {
            val bitrate = when {
                w >= 3840 -> 32_000_000
                w >= 2560 -> 20_000_000
                w >= 1920 -> 12_000_000
                else -> 7_500_000
            }
            val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, w, h).apply {
                setInteger(
                    MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface,
                )
                setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
                setInteger(MediaFormat.KEY_FRAME_RATE, safeFps)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
            }

            codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC).apply {
                configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            }
            inputSurface = codec.createInputSurface()
            codec.start()

            muxer = MediaMuxer(tmp.absolutePath, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)

            val beginRes = controller.runOnRenderThread { ptr ->
                savedCamId = NovaNativeBridge.nativeGetActiveCameraNode(ptr)
                val tl = NovaNativeBridge.nativeGetTimelineState(ptr)
                if (tl.size >= 4) savedFrame = tl[3]
                if (config.cameraNodeId != savedCamId) {
                    NovaNativeBridge.nativeSetActiveCameraNode(ptr, config.cameraNodeId)
                }
                NovaNativeBridge.nativeBeginVideoSurface(
                    ptr,
                    inputSurface!!,
                    w,
                    h,
                    config.quality.level,
                    config.aaEnabled,
                    config.msaaSamples,
                    config.ssaoEnabled,
                    config.bloomEnabled,
                    config.dofEnabled,
                    config.dofFocusDistance,
                    config.dofMaxAperture,
                    config.dofCocScale,
                    config.toneMapping.code,
                )
            }
            if (beginRes.isFailure || beginRes.getOrDefault(false) != true) {
                error("Failed to attach Filament offscreen Surface to H.264 encoder")
            }
            surfaceBound = true

            val bufferInfo = MediaCodec.BufferInfo()
            val frameStepNanos = 1_000_000_000L / safeFps.toLong()

            fun drainEncoder(endOfStream: Boolean) {
                if (endOfStream) {
                    codec?.signalEndOfInputStream()
                }
                val c = codec ?: return
                val m = muxer ?: return
                while (true) {
                    val outIdx = c.dequeueOutputBuffer(bufferInfo, if (endOfStream) 25_000L else 5_000L)
                    if (outIdx == MediaCodec.INFO_TRY_AGAIN_LATER) {
                        if (!endOfStream) break
                    } else if (outIdx == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                        if (muxerStarted) continue
                        trackIndex = m.addTrack(c.outputFormat)
                        m.start()
                        muxerStarted = true
                    } else if (outIdx >= 0) {
                        val encoded = c.getOutputBuffer(outIdx)
                        if (encoded != null && (bufferInfo.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG) == 0 && bufferInfo.size > 0) {
                            if (muxerStarted) {
                                encoded.position(bufferInfo.offset)
                                encoded.limit(bufferInfo.offset + bufferInfo.size)
                                m.writeSampleData(trackIndex, encoded, bufferInfo)
                            }
                        }
                        val eos = (bufferInfo.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0
                        c.releaseOutputBuffer(outIdx, false)
                        if (eos) break
                    }
                }
            }

            for (i in 0 until totalFrames) {
                if (isCancelled()) {
                    error("Render cancelled by user")
                }
                val frameNum = (fStart + i).toFloat()
                val ptsNanos = (i.toLong() + 1L) * frameStepNanos
                onProgress(i, totalFrames, "Rendering frame ${i + 1} / $totalFrames (${w}x${h})")

                val okFrame = controller.runOnRenderThread { ptr ->
                    NovaNativeBridge.nativeSetCurrentFrame(ptr, frameNum)
                    NovaNativeBridge.nativeRenderVideoFrame(ptr, ptsNanos, w, h)
                }.getOrDefault(false)
                if (!okFrame) {
                    error("Offscreen video frame render failed at frame ${fStart + i}")
                }
                drainEncoder(endOfStream = false)
            }

            onProgress(totalFrames, totalFrames, "Finalizing MP4 container...")
            drainEncoder(endOfStream = true)

            if (surfaceBound) {
                controller.runOnRenderThread { ptr ->
                    NovaNativeBridge.nativeEndVideoSurface(ptr)
                    NovaNativeBridge.nativeSetCurrentFrame(ptr, savedFrame)
                    if (config.cameraNodeId != savedCamId) {
                        NovaNativeBridge.nativeSetActiveCameraNode(ptr, savedCamId)
                    }
                }
                surfaceBound = false
            }

            if (muxerStarted) {
                runCatching { muxer?.stop() }
                muxerStarted = false
            }
            muxer?.release()
            muxer = null
            codec?.stop()
            codec?.release()
            codec = null
            inputSurface?.release()
            inputSurface = null

            if (!tmp.renameTo(dst)) {
                tmp.copyTo(dst, overwrite = true)
                tmp.delete()
            }

            val item = SavedRenderItem(
                fileName = dst.name,
                file = dst,
                isVideo = true,
                sizeBytes = dst.length(),
                modifiedAtMs = dst.lastModified(),
            )
            Result.success(Pair(item, preflight))
        } catch (t: Throwable) {
            if (surfaceBound) {
                runCatching {
                    controller.runOnRenderThread { ptr ->
                        NovaNativeBridge.nativeEndVideoSurface(ptr)
                        NovaNativeBridge.nativeSetCurrentFrame(ptr, savedFrame)
                        if (config.cameraNodeId != savedCamId) {
                            NovaNativeBridge.nativeSetActiveCameraNode(ptr, savedCamId)
                        }
                    }
                }
            }
            if (muxerStarted) {
                runCatching { muxer?.stop() }
            }
            runCatching { muxer?.release() }
            runCatching { codec?.stop() }
            runCatching { codec?.release() }
            runCatching { inputSurface?.release() }
            if (tmp.exists()) tmp.delete()
            Result.failure(t)
        }
    }

    private fun sanitizePrefix(title: String): String =
        title.lowercase(Locale.US)
            .replace(Regex("[^a-z0-9]+"), "_")
            .trim('_')
            .ifBlank { "nova_render" }
}
