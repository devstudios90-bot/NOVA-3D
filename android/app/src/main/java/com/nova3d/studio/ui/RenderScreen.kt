package com.nova3d.studio.ui

import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.FileProvider
import com.nova3d.studio.render.DeviceRenderCapabilities
import com.nova3d.studio.render.OffscreenRenderEngine
import com.nova3d.studio.render.RenderImageFormat
import com.nova3d.studio.render.RenderJobConfig
import com.nova3d.studio.render.RenderQualityPreset
import com.nova3d.studio.render.RenderResolutionPreset
import com.nova3d.studio.render.RenderToneMapping
import com.nova3d.studio.render.SavedRenderItem
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.ViewportController
import java.io.File
import java.util.Locale
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.roundToInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun RenderScreen(
    controller: ViewportController,
    onExportGlb: () -> Unit,
    onExportGltf: () -> Unit,
    onExportObj: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var renderVideoMode by remember { mutableStateOf(false) }
    var resolution by remember { mutableStateOf(RenderResolutionPreset.P1080) }
    var quality by remember { mutableStateOf(RenderQualityPreset.HIGH) }
    var aaEnabled by remember { mutableStateOf(true) }
    var msaaSamples by remember { mutableIntStateOf(4) }
    var ssaoEnabled by remember { mutableStateOf(true) }
    var bloomEnabled by remember { mutableStateOf(true) }
    var dofEnabled by remember { mutableStateOf(false) }
    var dofFocusDist by remember { mutableFloatStateOf(4.2f) }
    var dofMaxAperture by remember { mutableFloatStateOf(0.015f) }
    var toneMapping by remember { mutableStateOf(RenderToneMapping.ACES) }
    var imageFormat by remember { mutableStateOf(RenderImageFormat.PNG) }

    val cameras = remember(controller.sceneNodes) {
        controller.sceneNodes.filter { it.kind == 3 }
    }
    var selectedCameraId by remember {
        mutableStateOf(controller.timeline.activeCameraNodeId.takeIf { id -> cameras.any { it.id == id } } ?: 0L)
    }

    var startFrame by remember(controller.timeline.startFrame) { mutableIntStateOf(controller.timeline.startFrame) }
    var endFrame by remember(controller.timeline.endFrame) { mutableIntStateOf(controller.timeline.endFrame) }
    var fps by remember(controller.timeline.fps) { mutableFloatStateOf(controller.timeline.fps) }

    var caps by remember { mutableStateOf<DeviceRenderCapabilities?>(null) }
    var savedRenders by remember { mutableStateOf<List<SavedRenderItem>>(emptyList()) }
    var selectedPreviewItem by remember { mutableStateOf<SavedRenderItem?>(null) }
    var previewBitmap by remember { mutableStateOf<Bitmap?>(null) }

    var isRendering by remember { mutableStateOf(false) }
    var cancelFlag by remember { mutableStateOf(AtomicBoolean(false)) }
    var progressFraction by remember { mutableFloatStateOf(0f) }
    var progressLabel by remember { mutableStateOf("") }
    var lastNotice by remember { mutableStateOf<String?>(null) }
    var pendingSaveFile by remember { mutableStateOf<File?>(null) }

    fun reloadGallery(selectItem: SavedRenderItem? = null) {
        val list = OffscreenRenderEngine.listSavedRenders(context)
        savedRenders = list
        if (selectItem != null) {
            selectedPreviewItem = selectItem
        } else if (selectedPreviewItem == null || list.none { it.fileName == selectedPreviewItem?.fileName }) {
            selectedPreviewItem = list.firstOrNull()
        }
    }

    LaunchedEffect(Unit) {
        caps = withContext(Dispatchers.IO) { OffscreenRenderEngine.probeCapabilities(context) }
        reloadGallery()
    }

    LaunchedEffect(selectedPreviewItem?.fileName, selectedPreviewItem?.modifiedAtMs) {
        val target = selectedPreviewItem
        if (target == null || !target.file.exists()) {
            previewBitmap = null
            return@LaunchedEffect
        }
        previewBitmap = withContext(Dispatchers.IO) {
            runCatching {
                if (target.isVideo) {
                    val mmr = MediaMetadataRetriever()
                    try {
                        mmr.setDataSource(target.file.absolutePath)
                        mmr.getFrameAtTime(0L, MediaMetadataRetriever.OPTION_CLOSEST_SYNC)
                    } finally {
                        mmr.release()
                    }
                } else {
                    BitmapFactory.decodeFile(target.file.absolutePath)
                }
            }.getOrNull()
        }
    }

    val saveToDeviceLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("*/*"),
    ) { dstUri: Uri? ->
        val src = pendingSaveFile
        pendingSaveFile = null
        if (dstUri != null && src != null && src.exists()) {
            scope.launch {
                OffscreenRenderEngine.exportRenderToUri(context, src, dstUri)
                    .onSuccess { controller.showStatus("Exported ${src.name} to device storage") }
                    .onFailure { err -> controller.showError("Save failed: ${err.message}") }
            }
        }
    }

    val preflight = remember(caps, resolution, renderVideoMode) {
        caps?.let { OffscreenRenderEngine.preflightResolution(it, resolution, renderVideoMode) }
    }

    fun launchRender() {
        if (isRendering) return
        val jobCfg = RenderJobConfig(
            resolution = resolution,
            quality = quality,
            aaEnabled = aaEnabled,
            msaaSamples = msaaSamples,
            ssaoEnabled = ssaoEnabled,
            bloomEnabled = bloomEnabled,
            dofEnabled = dofEnabled,
            dofFocusDistance = dofFocusDist,
            dofMaxAperture = dofMaxAperture,
            dofCocScale = 1.0f,
            toneMapping = toneMapping,
            imageFormat = imageFormat,
            cameraNodeId = selectedCameraId,
        )

        val flag = AtomicBoolean(false)
        cancelFlag = flag
        isRendering = true
        lastNotice = null
        progressFraction = 0.05f
        progressLabel = if (renderVideoMode) "Preparing H.264 encoder..." else "Rendering clean Filament pass..."

        scope.launch {
            if (!renderVideoMode) {
                OffscreenRenderEngine.renderStillToFile(context, controller, jobCfg)
                    .onSuccess { (item, pf) ->
                        reloadGallery(selectItem = item)
                        lastNotice = pf.warningMessage
                        controller.showStatus("Saved still ${item.fileName} (${formatBytes(item.sizeBytes)})")
                    }
                    .onFailure { err ->
                        controller.showError("Still render failed: ${err.message ?: "unknown error"}")
                    }
            } else {
                OffscreenRenderEngine.renderVideoToMp4(
                    context = context,
                    controller = controller,
                    config = jobCfg,
                    startFrame = startFrame,
                    endFrame = endFrame,
                    fps = fps,
                    isCancelled = { flag.get() },
                    onProgress = { done, total, status ->
                        progressFraction = if (total > 0) done.toFloat() / total.toFloat() else 0f
                        progressLabel = status
                    },
                )
                    .onSuccess { (item, pf) ->
                        reloadGallery(selectItem = item)
                        lastNotice = pf.warningMessage
                        controller.showStatus("Rendered MP4 ${item.fileName} (${formatBytes(item.sizeBytes)})")
                    }
                    .onFailure { err ->
                        val msg = err.message ?: "unknown error"
                        if (msg.contains("cancelled", ignoreCase = true)) {
                            controller.showStatus("Video render cancelled.")
                        } else {
                            controller.showError("Video render failed: $msg")
                        }
                    }
            }
            isRendering = false
            progressFraction = 0f
            progressLabel = ""
        }
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(NovaColors.CanvasBg)
            .verticalScroll(rememberScrollState())
            .padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        // 1. Clean Offscreen Render Preview Card
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column {
                        Text(
                            text = "Final Render Output",
                            color = NovaColors.TextPrimary,
                            fontSize = 15.sp,
                            fontWeight = FontWeight.SemiBold,
                        )
                        Text(
                            text = "Clean PBR Filament pass (grid, axes, outlines & gizmos stripped)",
                            color = NovaColors.TextSecondary,
                            fontSize = 11.sp,
                        )
                    }
                    if (selectedPreviewItem != null) {
                        val item = selectedPreviewItem!!
                        Surface(
                            color = if (item.isVideo) NovaColors.AccentBlueSoft else NovaColors.CardBg,
                            shape = RoundedCornerShape(6.dp),
                            border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        ) {
                            Text(
                                text = if (item.isVideo) "MP4" else item.fileName.substringAfterLast('.').uppercase(Locale.US),
                                color = NovaColors.TextPrimary,
                                fontSize = 10.sp,
                                fontFamily = FontFamily.Monospace,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                            )
                        }
                    }
                }

                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .aspectRatio(16f / 9f)
                        .clip(RoundedCornerShape(8.dp))
                        .background(Color(0xFF0A0C10))
                        .border(1.dp, NovaColors.PanelBorder, RoundedCornerShape(8.dp)),
                    contentAlignment = Alignment.Center,
                ) {
                    val bmp = previewBitmap
                    if (bmp != null) {
                        Image(
                            bitmap = bmp.asImageBitmap(),
                            contentDescription = "Rendered Preview",
                            contentScale = ContentScale.Fit,
                            modifier = Modifier.fillMaxSize(),
                        )
                        val item = selectedPreviewItem
                        if (item != null) {
                            Surface(
                                color = Color(0xAA0D1016),
                                shape = RoundedCornerShape(6.dp),
                                modifier = Modifier
                                    .align(Alignment.BottomStart)
                                    .padding(8.dp),
                            ) {
                                Text(
                                    text = "${item.fileName} · ${formatBytes(item.sizeBytes)} · ${bmp.width}x${bmp.height}",
                                    color = NovaColors.TextPrimary,
                                    fontSize = 11.sp,
                                    fontFamily = FontFamily.Monospace,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                                )
                            }
                        }
                    } else {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(6.dp),
                            modifier = Modifier.padding(16.dp),
                        ) {
                            Text(
                                text = "No render selected yet",
                                color = NovaColors.TextSecondary,
                                fontSize = 13.sp,
                                fontWeight = FontWeight.Medium,
                            )
                            Text(
                                text = "Configure resolution & post-processing below, then tap Render.",
                                color = NovaColors.TextMuted,
                                fontSize = 11.sp,
                            )
                        }
                    }
                }

                if (selectedPreviewItem != null) {
                    val item = selectedPreviewItem!!
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        OutlinedButton(
                            onClick = {
                                pendingSaveFile = item.file
                                saveToDeviceLauncher.launch(item.fileName)
                            },
                        ) {
                            Text("Save to Device", fontSize = 12.sp, color = NovaColors.TextPrimary)
                        }
                        OutlinedButton(
                            onClick = {
                                runCatching { OffscreenRenderEngine.shareRender(context, item.file) }
                                    .onFailure { controller.showError("Share failed: ${it.message}") }
                            },
                        ) {
                            Text("Share", fontSize = 12.sp, color = NovaColors.TextPrimary)
                        }
                        if (item.isVideo) {
                            OutlinedButton(
                                onClick = {
                                    runCatching {
                                        val uri = FileProvider.getUriForFile(
                                            context,
                                            "${context.packageName}.fileprovider",
                                            item.file,
                                        )
                                        val intent = Intent(Intent.ACTION_VIEW).apply {
                                            setDataAndType(uri, "video/mp4")
                                            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                                        }
                                        context.startActivity(intent)
                                    }.onFailure {
                                        controller.showError("No video player app found: ${it.message}")
                                    }
                                },
                            ) {
                                Text("Play MP4", fontSize = 12.sp, color = NovaColors.AccentBlue)
                            }
                        }
                    }
                }
            }
        }

        // 2. Output Type, Resolution & Pre-flight Memory/Encoder Check (RN-1, RN-5, RN-6, RN-7)
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(
                    text = "Output Target & Resolution",
                    color = NovaColors.TextPrimary,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.SemiBold,
                )

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    RenderModeChip(
                        label = "Still Image",
                        selected = !renderVideoMode,
                        enabled = !isRendering,
                        onClick = { renderVideoMode = false },
                    )
                    RenderModeChip(
                        label = "Animation MP4",
                        selected = renderVideoMode,
                        enabled = !isRendering,
                        onClick = { renderVideoMode = true },
                    )
                }

                Text("Resolution Preset", color = NovaColors.TextSecondary, fontSize = 12.sp)
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    RenderResolutionPreset.entries.forEach { preset ->
                        RenderModeChip(
                            label = "${preset.label} (${preset.width}x${preset.height})",
                            selected = resolution == preset,
                            enabled = !isRendering,
                            onClick = { resolution = preset },
                        )
                    }
                }

                // Pre-flight hardware & OOM safety banner (RN-7)
                if (caps != null && preflight != null) {
                    val pf = preflight
                    val c = caps!!
                    val warn = pf.wasDownscaled
                    Surface(
                        color = if (warn) Color(0xFF2B2212) else NovaColors.CardBg,
                        shape = RoundedCornerShape(8.dp),
                        border = BorderStroke(
                            1.dp,
                            if (warn) NovaColors.WarningAmber else NovaColors.PanelBorder,
                        ),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Column(
                            modifier = Modifier.padding(10.dp),
                            verticalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Text(
                                text = "Device Check: ${c.availableRamMb} MiB free RAM · H.264 Max ${c.maxVideoWidth}x${c.maxVideoHeight}" +
                                    (c.avcEncoderName?.let { " ($it)" } ?: ""),
                                color = if (warn) NovaColors.WarningAmber else NovaColors.SuccessGreen,
                                fontSize = 11.sp,
                                fontFamily = FontFamily.Monospace,
                            )
                            if (pf.warningMessage != null) {
                                Text(
                                    text = pf.warningMessage,
                                    color = NovaColors.TextPrimary,
                                    fontSize = 11.sp,
                                )
                            }
                        }
                    }
                }

                if (!renderVideoMode) {
                    Text("Image Format", color = NovaColors.TextSecondary, fontSize = 12.sp)
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        RenderImageFormat.entries.forEach { fmt ->
                            RenderModeChip(
                                label = fmt.label,
                                selected = imageFormat == fmt,
                                enabled = !isRendering,
                                onClick = { imageFormat = fmt },
                            )
                        }
                    }
                } else {
                    Text(
                        text = "Frame Range: $startFrame .. $endFrame @ ${fps.roundToInt()} fps (${((endFrame - startFrame + 1).coerceAtLeast(1) / fps.coerceAtLeast(1f)).let { "%.2f".format(Locale.US, it) }}s)",
                        color = NovaColors.TextSecondary,
                        fontSize = 12.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Start F$startFrame", color = NovaColors.TextMuted, fontSize = 11.sp)
                            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                RenderModeChip("-5", false, !isRendering) {
                                    startFrame = (startFrame - 5).coerceIn(1, endFrame)
                                }
                                RenderModeChip("+5", false, !isRendering) {
                                    startFrame = (startFrame + 5).coerceIn(1, endFrame)
                                }
                            }
                        }
                        Column(modifier = Modifier.weight(1f)) {
                            Text("End F$endFrame", color = NovaColors.TextMuted, fontSize = 11.sp)
                            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                RenderModeChip("-10", false, !isRendering) {
                                    endFrame = (endFrame - 10).coerceAtLeast(startFrame)
                                }
                                RenderModeChip("+10", false, !isRendering) {
                                    endFrame = (endFrame + 10).coerceAtMost(2400)
                                }
                            }
                        }
                        Column(modifier = Modifier.weight(1f)) {
                            Text("FPS", color = NovaColors.TextMuted, fontSize = 11.sp)
                            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                listOf(24f, 30f, 60f).forEach { f ->
                                    RenderModeChip("${f.toInt()}", fps == f, !isRendering) { fps = f }
                                }
                            }
                        }
                    }
                }

                Text("Camera Source", color = NovaColors.TextSecondary, fontSize = 12.sp)
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    RenderModeChip(
                        label = "Viewport Orbit",
                        selected = selectedCameraId == 0L,
                        enabled = !isRendering,
                        onClick = { selectedCameraId = 0L },
                    )
                    cameras.forEach { cam ->
                        RenderModeChip(
                            label = cam.name,
                            selected = selectedCameraId == cam.id,
                            enabled = !isRendering,
                            onClick = { selectedCameraId = cam.id },
                        )
                    }
                }
            }
        }

        // 3. Quality Presets, Post-Processing & Depth of Field (RN-2, RN-4)
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(
                    text = "Quality & Post-Processing",
                    color = NovaColors.TextPrimary,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.SemiBold,
                )

                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    RenderQualityPreset.entries.forEach { q ->
                        RenderModeChip(
                            label = q.label,
                            selected = quality == q,
                            enabled = !isRendering,
                            onClick = {
                                quality = q
                                msaaSamples = q.defaultMsaa
                                ssaoEnabled = q.defaultSsao
                                bloomEnabled = q.defaultBloom
                            },
                        )
                    }
                }

                RenderSwitchRow("Anti-Aliasing (FXAA + ${msaaSamples}x MSAA)", aaEnabled, !isRendering) {
                    aaEnabled = it
                }
                RenderSwitchRow("Ambient Occlusion (SSAO / GTAO)", ssaoEnabled, !isRendering) {
                    ssaoEnabled = it
                }
                RenderSwitchRow("HDR Bloom", bloomEnabled, !isRendering) {
                    bloomEnabled = it
                }
                RenderSwitchRow("Depth of Field (Bokeh Blur)", dofEnabled, !isRendering) {
                    dofEnabled = it
                }

                if (dofEnabled) {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(
                            text = "Focus Distance: ${"%.2f".format(Locale.US, dofFocusDist)} m",
                            color = NovaColors.TextSecondary,
                            fontSize = 11.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                        Slider(
                            value = dofFocusDist,
                            onValueChange = { dofFocusDist = it },
                            valueRange = 0.5f..25.0f,
                            enabled = !isRendering,
                            colors = SliderDefaults.colors(
                                thumbColor = NovaColors.AccentBlue,
                                activeTrackColor = NovaColors.AccentBlue,
                            ),
                        )
                        Text(
                            text = "Aperture Blur Strength: ${"%.3f".format(Locale.US, dofMaxAperture)}",
                            color = NovaColors.TextSecondary,
                            fontSize = 11.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                        Slider(
                            value = dofMaxAperture,
                            onValueChange = { dofMaxAperture = it },
                            valueRange = 0.002f..0.08f,
                            enabled = !isRendering,
                            colors = SliderDefaults.colors(
                                thumbColor = NovaColors.AccentBlue,
                                activeTrackColor = NovaColors.AccentBlue,
                            ),
                        )
                    }
                }

                Text("Tone Mapping", color = NovaColors.TextSecondary, fontSize = 12.sp)
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    RenderToneMapping.entries.forEach { tm ->
                        RenderModeChip(
                            label = tm.label,
                            selected = toneMapping == tm,
                            enabled = !isRendering,
                            onClick = { toneMapping = tm },
                        )
                    }
                }

                if (lastNotice != null) {
                    Text(
                        text = lastNotice!!,
                        color = NovaColors.WarningAmber,
                        fontSize = 11.sp,
                    )
                }

                if (isRendering) {
                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(
                            text = progressLabel,
                            color = NovaColors.TextPrimary,
                            fontSize = 12.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                        LinearProgressIndicator(
                            progress = { progressFraction.coerceIn(0f, 1f) },
                            modifier = Modifier.fillMaxWidth().height(6.dp),
                            color = NovaColors.AccentBlue,
                            trackColor = NovaColors.CardBg,
                        )
                        OutlinedButton(
                            onClick = { cancelFlag.set(true) },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text("Cancel Render", color = NovaColors.ErrorRed)
                        }
                    }
                } else {
                    Button(
                        onClick = { launchRender() },
                        colors = ButtonDefaults.buttonColors(containerColor = NovaColors.AccentBlue),
                        modifier = Modifier.fillMaxWidth().height(46.dp),
                    ) {
                        val effectiveLabel = preflight?.let { "${it.effectiveWidth}x${it.effectiveHeight}" } ?: resolution.label
                        Text(
                            text = if (renderVideoMode) {
                                "Render MP4 Video ($effectiveLabel)"
                            } else {
                                "Render Still Frame ($effectiveLabel)"
                            },
                            fontWeight = FontWeight.SemiBold,
                            fontSize = 14.sp,
                        )
                    }
                }
            }
        }

        // 4. 3D Asset Exchange Shortcuts (FM-5, FM-6, FM-7)
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(
                    text = "Export Scene as 3D Exchange Format",
                    color = NovaColors.TextPrimary,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    OutlinedButton(onClick = onExportGlb, modifier = Modifier.weight(1f)) {
                        Text("Export .GLB", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                    OutlinedButton(onClick = onExportGltf, modifier = Modifier.weight(1f)) {
                        Text("Export .glTF", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                    OutlinedButton(onClick = onExportObj, modifier = Modifier.weight(1f)) {
                        Text("Export .OBJ", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                }
            }
        }

        // 5. Saved Renders Gallery
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(
                    text = "Saved Renders (${savedRenders.size})",
                    color = NovaColors.TextPrimary,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                if (savedRenders.isEmpty()) {
                    Text(
                        text = "Rendered stills and MP4 clips saved on device will appear here.",
                        color = NovaColors.TextMuted,
                        fontSize = 12.sp,
                    )
                } else {
                    savedRenders.forEach { item ->
                        val selected = selectedPreviewItem?.fileName == item.fileName
                        Surface(
                            color = if (selected) NovaColors.AccentBlueSoft else NovaColors.CardBg,
                            shape = RoundedCornerShape(8.dp),
                            border = BorderStroke(
                                1.dp,
                                if (selected) NovaColors.AccentBlue else NovaColors.PanelBorder,
                            ),
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { selectedPreviewItem = item },
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(
                                        text = item.fileName,
                                        color = NovaColors.TextPrimary,
                                        fontSize = 12.sp,
                                        fontFamily = FontFamily.Monospace,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    Text(
                                        text = "${if (item.isVideo) "Video MP4" else "Still Image"} · ${formatBytes(item.sizeBytes)}",
                                        color = NovaColors.TextSecondary,
                                        fontSize = 11.sp,
                                    )
                                }
                                Spacer(Modifier.width(8.dp))
                                TextButton(
                                    onClick = {
                                        pendingSaveFile = item.file
                                        saveToDeviceLauncher.launch(item.fileName)
                                    },
                                ) {
                                    Text("Save", fontSize = 12.sp, color = NovaColors.AccentBlue)
                                }
                                TextButton(
                                    onClick = {
                                        OffscreenRenderEngine.deleteSavedRender(item.file)
                                        reloadGallery()
                                    },
                                ) {
                                    Text("Del", fontSize = 12.sp, color = NovaColors.ErrorRed)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun RenderModeChip(
    label: String,
    selected: Boolean,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    Surface(
        color = if (selected) NovaColors.AccentBlueSoft else NovaColors.CardBg,
        shape = RoundedCornerShape(7.dp),
        border = BorderStroke(1.dp, if (selected) NovaColors.AccentBlue else NovaColors.PanelBorder),
        modifier = Modifier.clickable(enabled = enabled, onClick = onClick),
    ) {
        Text(
            text = label,
            color = if (!enabled) NovaColors.TextMuted else if (selected) NovaColors.TextPrimary else NovaColors.TextSecondary,
            fontSize = 12.sp,
            fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
        )
    }
}

@Composable
private fun RenderSwitchRow(
    label: String,
    checked: Boolean,
    enabled: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(text = label, color = NovaColors.TextSecondary, fontSize = 13.sp, modifier = Modifier.weight(1f))
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            enabled = enabled,
        )
    }
}

private fun formatBytes(bytes: Long): String = when {
    bytes >= 1024L * 1024L -> "%.2f MB".format(Locale.US, bytes.toDouble() / (1024.0 * 1024.0))
    bytes >= 1024L -> "%.1f KB".format(Locale.US, bytes.toDouble() / 1024.0)
    else -> "$bytes B"
}
