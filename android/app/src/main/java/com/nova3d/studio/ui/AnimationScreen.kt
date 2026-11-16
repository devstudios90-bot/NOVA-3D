package com.nova3d.studio.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.AnimTrackUi
import com.nova3d.studio.viewport.InterpolationUiMode
import com.nova3d.studio.viewport.KeyframeUi
import com.nova3d.studio.viewport.ViewportController
import com.nova3d.studio.viewport.ViewportSurface
import java.util.Locale
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.roundToInt

private enum class AnimPaneTab(val label: String) {
    DopeSheet("Dope Sheet"),
    GraphEditor("Graph Editor"),
}

@Composable
fun AnimationScreen(
    controller: ViewportController,
    isLifecycleActive: Boolean,
) {
    val tl = controller.timeline
    val tracks = controller.animTracks
    val activeTrack = tracks.firstOrNull { it.id == controller.selectedTrackId } ?: tracks.firstOrNull()

    var activePane by remember { mutableStateOf(AnimPaneTab.DopeSheet) }
    var zoomScale by remember { mutableFloatStateOf(1.0f) }
    var keyMenuExpanded by remember { mutableStateOf(false) }
    var interpMenuExpanded by remember { mutableStateOf(false) }
    var camMenuExpanded by remember { mutableStateOf(false) }
    var settingsDialogOpen by remember { mutableStateOf(false) }
    var editKeyDialogTarget by remember { mutableStateOf<Pair<AnimTrackUi, KeyframeUi>?>(null) }

    val cameras = remember(controller.sceneNodes) {
        controller.sceneNodes.filter { it.kind == 3 }
    }
    val selectedNode = controller.selectedNodeDetails

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(NovaColors.CanvasBg),
    ) {
        // Top live 3D viewport (42% height)
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .weight(0.42f)
                .background(NovaColors.CanvasBg),
        ) {
            ViewportSurface(
                controller = controller,
                isActive = isLifecycleActive,
                modifier = Modifier.fillMaxSize(),
            )

            // Top viewport overlay actions
            Row(
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .fillMaxWidth()
                    .padding(6.dp)
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(5.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                // Orbit / Active Camera View toggle (PRD 8.7 / AN-3)
                Box {
                    val activeCam = cameras.firstOrNull { it.id == tl.activeCameraNodeId }
                    AnimChip(
                        label = if (activeCam != null) "Cam: ${activeCam.name} ▾" else "Orbit View ▾",
                        active = activeCam != null,
                        onClick = {
                            if (cameras.size <= 1) {
                                controller.toggleCameraView()
                            } else {
                                camMenuExpanded = true
                            }
                        },
                    )
                    DropdownMenu(
                        expanded = camMenuExpanded,
                        onDismissRequest = { camMenuExpanded = false },
                        containerColor = NovaColors.PanelBg,
                    ) {
                        DropdownMenuItem(
                            text = {
                                Text(
                                    "Orbit View",
                                    color = if (tl.activeCameraNodeId == 0L) NovaColors.AccentBlue else NovaColors.TextPrimary,
                                    fontSize = 12.sp,
                                )
                            },
                            onClick = {
                                camMenuExpanded = false
                                controller.setActiveCameraNode(0L)
                            },
                        )
                        cameras.forEach { cam ->
                            DropdownMenuItem(
                                text = {
                                    Text(
                                        "Cam: ${cam.name}",
                                        color = if (tl.activeCameraNodeId == cam.id) NovaColors.AccentBlue else NovaColors.TextPrimary,
                                        fontSize = 12.sp,
                                    )
                                },
                                onClick = {
                                    camMenuExpanded = false
                                    controller.setActiveCameraNode(cam.id)
                                },
                            )
                        }
                    }
                }

                // + Key menu (Location / Rotation / Scale / Light / Camera / Material)
                Box {
                    AnimChip(
                        label = "+ Key ◆ ▾",
                        active = keyMenuExpanded,
                        accent = Color(0xFFD97706),
                        onClick = { keyMenuExpanded = true },
                    )
                    DropdownMenu(
                        expanded = keyMenuExpanded,
                        onDismissRequest = { keyMenuExpanded = false },
                        containerColor = NovaColors.PanelBg,
                    ) {
                        DropdownMenuItem(
                            text = { Text("Loc + Rot + Scale (All TRS)", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                            enabled = selectedNode != null,
                            onClick = {
                                keyMenuExpanded = false
                                controller.keySelectedNodeTrs(7)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Location (X, Y, Z)", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                            enabled = selectedNode != null,
                            onClick = {
                                keyMenuExpanded = false
                                controller.keySelectedNodeTrs(1)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Rotation (X, Y, Z)", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                            enabled = selectedNode != null,
                            onClick = {
                                keyMenuExpanded = false
                                controller.keySelectedNodeTrs(2)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Scale (X, Y, Z)", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                            enabled = selectedNode != null,
                            onClick = {
                                keyMenuExpanded = false
                                controller.keySelectedNodeTrs(4)
                            },
                        )
                        if (selectedNode?.kind == 2) {
                            DropdownMenuItem(
                                text = { Text("Light Parameters", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                onClick = {
                                    keyMenuExpanded = false
                                    controller.keySelectedLight()
                                },
                            )
                        }
                        if (selectedNode?.kind == 3) {
                            DropdownMenuItem(
                                text = { Text("Camera (Loc/Rot/FOV)", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                onClick = {
                                    keyMenuExpanded = false
                                    controller.keySelectedCamera()
                                },
                            )
                        }
                        val matId = selectedNode?.materialId?.takeIf { it > 0L } ?: controller.selectedMaterialId
                        if (matId > 0L) {
                            DropdownMenuItem(
                                text = { Text("Material (Color/PBR/Emissive)", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                onClick = {
                                    keyMenuExpanded = false
                                    controller.keyMaterial(matId)
                                },
                            )
                        }
                    }
                }

                AnimChip(
                    label = "Undo",
                    active = false,
                    enabled = controller.canUndo,
                    onClick = { controller.undo() },
                )
                AnimChip(
                    label = "Redo",
                    active = false,
                    enabled = controller.canRedo,
                    onClick = { controller.redo() },
                )
            }

            // Bottom-left readout inside viewport
            Surface(
                color = NovaColors.PanelBg.copy(alpha = 0.85f),
                shape = RoundedCornerShape(6.dp),
                border = BorderStroke(1.dp, NovaColors.PanelBorder),
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .padding(6.dp),
            ) {
                val frameLabel = String.format(
                    Locale.US,
                    "F %.1f / %d @ %.0f fps · %s · %d tracks",
                    tl.currentFrame,
                    tl.endFrame,
                    tl.fps,
                    if (selectedNode != null) {
                        controller.sceneNodes.firstOrNull { it.id == selectedNode.id }?.name ?: "Sel"
                    } else "No Sel",
                    tracks.size,
                )
                Text(
                    text = frameLabel,
                    color = NovaColors.TextSecondary,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                )
            }
        }

        HorizontalDivider(color = NovaColors.PanelBorder)

        // Transport & Playback Bar (AN-1, AN-6)
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(NovaColors.PanelBg)
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 8.dp, vertical = 5.dp),
            horizontalArrangement = Arrangement.spacedBy(5.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            AnimChip(label = "|◀", active = false, onClick = { controller.rewindToStart() })
            AnimChip(label = "◀◆", active = false, onClick = { controller.stepKeyframe(-1) })
            AnimChip(
                label = if (tl.playing) "⏸ Pause" else "▶ Play",
                active = tl.playing,
                accent = if (tl.playing) Color(0xFF16A34A) else NovaColors.AccentBlue,
                onClick = { controller.togglePlaying() },
            )
            AnimChip(label = "◆▶", active = false, onClick = { controller.stepKeyframe(1) })

            AnimChip(
                label = if (tl.looping) "Loop: On" else "Loop: Off",
                active = tl.looping,
                onClick = { controller.updateTimelineConfig(looping = !tl.looping) },
            )
            AnimChip(
                label = if (tl.autoKey) "● Auto-Key" else "○ Auto-Key",
                active = tl.autoKey,
                accent = Color(0xFFDC2626),
                onClick = { controller.updateTimelineConfig(autoKey = !tl.autoKey) },
            )
            AnimChip(
                label = "${tl.startFrame}..${tl.endFrame} (${tl.fps.toInt()}fps)",
                active = false,
                onClick = { settingsDialogOpen = true },
            )
        }

        HorizontalDivider(color = NovaColors.PanelBorder)

        // Editor sub-tabs & Zoom / Interpolation Bar
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(NovaColors.PanelBg)
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 8.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(5.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            AnimPaneTab.entries.forEach { tab ->
                AnimChip(
                    label = tab.label,
                    active = activePane == tab,
                    onClick = { activePane = tab },
                )
            }

            Spacer(modifier = Modifier.width(4.dp))

            Box {
                AnimChip(
                    label = "${controller.defaultInterpolation.label} ▾",
                    active = false,
                    onClick = { interpMenuExpanded = true },
                )
                DropdownMenu(
                    expanded = interpMenuExpanded,
                    onDismissRequest = { interpMenuExpanded = false },
                    containerColor = NovaColors.PanelBg,
                ) {
                    InterpolationUiMode.entries.forEach { mode ->
                        DropdownMenuItem(
                            text = {
                                Text(
                                    mode.label,
                                    color = if (controller.defaultInterpolation == mode) NovaColors.AccentBlue else NovaColors.TextPrimary,
                                    fontSize = 12.sp,
                                )
                            },
                            onClick = {
                                interpMenuExpanded = false
                                controller.defaultInterpolation = mode
                                val selTrack = activeTrack
                                val selKey = selTrack?.keyframes?.firstOrNull { it.id == controller.selectedKeyframeId }
                                if (selTrack != null && selKey != null) {
                                    controller.updateKeyframe(
                                        selTrack.id,
                                        selKey.id,
                                        selKey.frame,
                                        selKey.value,
                                        mode,
                                        selKey.inHandleX,
                                        selKey.inHandleY,
                                        selKey.outHandleX,
                                        selKey.outHandleY,
                                    )
                                }
                            },
                        )
                    }
                }
            }

            AnimChip(
                label = "−",
                active = false,
                onClick = { zoomScale = (zoomScale / 1.35f).coerceIn(0.4f, 6.0f) },
            )
            AnimChip(
                label = "1x",
                active = false,
                onClick = { zoomScale = 1.0f },
            )
            AnimChip(
                label = "+",
                active = false,
                onClick = { zoomScale = (zoomScale * 1.35f).coerceIn(0.4f, 6.0f) },
            )
        }

        HorizontalDivider(color = NovaColors.PanelBorder)

        // Bottom Workspace: Dope Sheet or Graph Editor (58% height)
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .weight(0.58f)
                .background(NovaColors.PanelBg),
        ) {
            when (activePane) {
                AnimPaneTab.DopeSheet -> DopeSheetEditor(
                    controller = controller,
                    zoomScale = zoomScale,
                    onEditKeyframe = { track, key -> editKeyDialogTarget = track to key },
                )

                AnimPaneTab.GraphEditor -> CurveGraphEditor(
                    controller = controller,
                    zoomScale = zoomScale,
                    activeTrack = activeTrack,
                    onEditKeyframe = { track, key -> editKeyDialogTarget = track to key },
                )
            }
        }
    }

    if (settingsDialogOpen) {
        TimelineSettingsDialog(
            startFrame = tl.startFrame,
            endFrame = tl.endFrame,
            fps = tl.fps,
            onDismiss = { settingsDialogOpen = false },
            onConfirm = { s, e, fps ->
                settingsDialogOpen = false
                controller.updateTimelineConfig(startFrame = s, endFrame = e, fps = fps)
            },
        )
    }

    val editTarget = editKeyDialogTarget
    if (editTarget != null) {
        EditKeyframeDialog(
            trackLabel = "${controller.trackTargetName(editTarget.first)} · ${editTarget.first.channelLabel()}",
            keyframe = editTarget.second,
            onDismiss = { editKeyDialogTarget = null },
            onDelete = {
                editKeyDialogTarget = null
                controller.deleteKeyframe(editTarget.first.id, editTarget.second.id)
            },
            onConfirm = { frame, value, interp ->
                editKeyDialogTarget = null
                controller.updateKeyframe(
                    editTarget.first.id,
                    editTarget.second.id,
                    frame,
                    value,
                    interp,
                    editTarget.second.inHandleX,
                    editTarget.second.inHandleY,
                    editTarget.second.outHandleX,
                    editTarget.second.outHandleY,
                )
            },
        )
    }
}

@Composable
private fun DopeSheetEditor(
    controller: ViewportController,
    zoomScale: Float,
    onEditKeyframe: (AnimTrackUi, KeyframeUi) -> Unit,
) {
    val tl = controller.timeline
    val tracks = controller.animTracks
    val hScroll = rememberScrollState()
    val frameCount = (tl.endFrame - tl.startFrame + 1).coerceAtLeast(2)
    val pxPerFrameDp = (16f * zoomScale).coerceIn(6f, 64f)
    val timelineWidthDp = (frameCount * pxPerFrameDp).dp

    val selTrack = tracks.firstOrNull { it.id == controller.selectedTrackId }
    val selKey = selTrack?.keyframes?.firstOrNull { it.id == controller.selectedKeyframeId }

    Column(modifier = Modifier.fillMaxSize()) {
        // Shared Ruler + Scrollable Tracks
        Row(modifier = Modifier.fillMaxWidth().background(NovaColors.CardBg)) {
            Box(
                modifier = Modifier
                    .width(126.dp)
                    .height(28.dp)
                    .border(0.5.dp, NovaColors.PanelBorder)
                    .padding(horizontal = 6.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                Text(
                    text = "Channel",
                    color = NovaColors.TextSecondary,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                )
            }
            Box(
                modifier = Modifier
                    .weight(1f)
                    .horizontalScroll(hScroll),
            ) {
                TimelineRulerCanvas(
                    startFrame = tl.startFrame,
                    endFrame = tl.endFrame,
                    currentFrame = tl.currentFrame,
                    pxPerFrameDp = pxPerFrameDp,
                    onScrubFrame = { controller.setCurrentFrame(it) },
                    modifier = Modifier
                        .width(timelineWidthDp)
                        .height(28.dp),
                )
            }
        }

        if (tracks.isEmpty()) {
            Box(
                modifier = Modifier.fillMaxSize().padding(16.dp),
                contentAlignment = Alignment.Center,
            ) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(
                        text = "No Keyframes Yet",
                        color = NovaColors.TextPrimary,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        text = "Select an object, move the playhead, and tap + Key ◆ or enable Auto-Key.",
                        color = NovaColors.TextSecondary,
                        fontSize = 12.sp,
                    )
                    AnimChip(
                        label = "+ Key Selected TRS @ F${tl.currentFrame.roundToInt()}",
                        active = true,
                        accent = Color(0xFFD97706),
                        onClick = { controller.keySelectedNodeTrs(7) },
                    )
                }
            }
        } else {
            LazyColumn(modifier = Modifier.weight(1f).fillMaxWidth()) {
                items(tracks, key = { it.id }) { track ->
                    val isTrackSelected = track.id == controller.selectedTrackId
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(42.dp)
                            .background(
                                if (isTrackSelected) NovaColors.AccentBlue.copy(alpha = 0.12f)
                                else Color.Transparent,
                            ),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(
                            modifier = Modifier
                                .width(126.dp)
                                .fillMaxHeight()
                                .border(0.5.dp, NovaColors.PanelBorder)
                                .clickable { controller.selectedTrackId = track.id }
                                .padding(horizontal = 6.dp, vertical = 2.dp),
                            verticalArrangement = Arrangement.Center,
                        ) {
                            Text(
                                text = track.channelLabel(),
                                color = if (isTrackSelected) NovaColors.AccentBlue else NovaColors.TextPrimary,
                                fontSize = 11.sp,
                                lineHeight = 13.sp,
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(
                                text = controller.trackTargetName(track),
                                color = NovaColors.TextMuted,
                                fontSize = 9.sp,
                                lineHeight = 11.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }

                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxHeight()
                                .horizontalScroll(hScroll),
                        ) {
                            TrackDiamondsCanvas(
                                track = track,
                                startFrame = tl.startFrame,
                                endFrame = tl.endFrame,
                                currentFrame = tl.currentFrame,
                                selectedKeyId = controller.selectedKeyframeId,
                                onSelectKeyframe = { kf ->
                                    controller.selectedTrackId = track.id
                                    controller.selectedKeyframeId = kf.id
                                    controller.setCurrentFrame(kf.frame.toFloat())
                                },
                                onMoveKeyframe = { kf, newFrame ->
                                    controller.updateKeyframe(
                                        track.id,
                                        kf.id,
                                        newFrame,
                                        kf.value,
                                        kf.interpolation,
                                        kf.inHandleX,
                                        kf.inHandleY,
                                        kf.outHandleX,
                                        kf.outHandleY,
                                    )
                                },
                                modifier = Modifier
                                    .width(timelineWidthDp)
                                    .fillMaxHeight(),
                            )
                        }
                    }
                }
            }

            if (selTrack != null && selKey != null) {
                HorizontalDivider(color = NovaColors.PanelBorder)
                SelectedKeyframeBar(
                    trackLabel = selTrack.channelLabel(),
                    keyframe = selKey,
                    onEdit = { onEditKeyframe(selTrack, selKey) },
                    onDelete = { controller.deleteKeyframe(selTrack.id, selKey.id) },
                )
            }
        }
    }
}

@Composable
private fun TimelineRulerCanvas(
    startFrame: Int,
    endFrame: Int,
    currentFrame: Float,
    pxPerFrameDp: Float,
    onScrubFrame: (Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    var sizePx by remember { mutableStateOf(IntSize.Zero) }
    val step = when {
        pxPerFrameDp >= 26f -> 1
        pxPerFrameDp >= 14f -> 5
        else -> 10
    }
    val textPaint = remember {
        android.graphics.Paint().apply {
            color = android.graphics.Color.argb(210, 175, 185, 200)
            textSize = 22f
            isAntiAlias = true
            typeface = android.graphics.Typeface.MONOSPACE
        }
    }
    Canvas(
        modifier = modifier
            .background(NovaColors.PanelBg)
            .onSizeChanged { sizePx = it }
            .pointerInput(startFrame, endFrame) {
                detectTapGestures { offset ->
                    val span = (endFrame - startFrame).coerceAtLeast(1)
                    val w = sizePx.width.toFloat().coerceAtLeast(1f)
                    val f = startFrame + (offset.x / w) * span
                    onScrubFrame(f.coerceIn(startFrame.toFloat(), endFrame.toFloat()))
                }
            }
            .pointerInput(startFrame, endFrame) {
                detectDragGestures { change, _ ->
                    change.consume()
                    val span = (endFrame - startFrame).coerceAtLeast(1)
                    val w = sizePx.width.toFloat().coerceAtLeast(1f)
                    val f = startFrame + (change.position.x / w) * span
                    onScrubFrame(f.coerceIn(startFrame.toFloat(), endFrame.toFloat()))
                }
            },
    ) {
        val span = (endFrame - startFrame).coerceAtLeast(1).toFloat()
        val w = size.width
        val h = size.height
        for (f in startFrame..endFrame) {
            val x = ((f - startFrame) / span) * w
            val major = (f % step == 0) || f == startFrame
            drawLine(
                color = if (major) NovaColors.TextSecondary else NovaColors.PanelBorder,
                start = Offset(x, if (major) h * 0.48f else h * 0.72f),
                end = Offset(x, h),
                strokeWidth = if (major) 2f else 1f,
            )
            if (major && x + 28f < w) {
                drawContext.canvas.nativeCanvas.drawText(
                    f.toString(),
                    x + 4f,
                    h * 0.42f,
                    textPaint,
                )
            }
        }
        val playX = ((currentFrame - startFrame) / span) * w
        drawLine(
            color = Color(0xFF38BDF8),
            start = Offset(playX, 0f),
            end = Offset(playX, h),
            strokeWidth = 3f,
        )
        val cap = Path().apply {
            moveTo(playX - 7f, 0f)
            lineTo(playX + 7f, 0f)
            lineTo(playX, 11f)
            close()
        }
        drawPath(cap, color = Color(0xFF38BDF8))
    }
}

@Composable
private fun TrackDiamondsCanvas(
    track: AnimTrackUi,
    startFrame: Int,
    endFrame: Int,
    currentFrame: Float,
    selectedKeyId: Long?,
    onSelectKeyframe: (KeyframeUi) -> Unit,
    onMoveKeyframe: (KeyframeUi, Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    var draggingKey by remember { mutableStateOf<KeyframeUi?>(null) }
    var dragFrame by remember { mutableIntStateOf(0) }
    var sizePx by remember { mutableStateOf(IntSize.Zero) }

    Canvas(
        modifier = modifier
            .border(0.5.dp, NovaColors.PanelBorder)
            .onSizeChanged { sizePx = it }
            .pointerInput(track.keyframes, startFrame, endFrame) {
                detectTapGestures { offset ->
                    val span = (endFrame - startFrame).coerceAtLeast(1).toFloat()
                    val w = sizePx.width.toFloat().coerceAtLeast(1f)
                    val nearest = track.keyframes.minByOrNull { kf ->
                        val kx = ((kf.frame - startFrame) / span) * w
                        abs(kx - offset.x)
                    }
                    if (nearest != null) {
                        val kx = ((nearest.frame - startFrame) / span) * w
                        if (abs(kx - offset.x) <= 28f) {
                            onSelectKeyframe(nearest)
                        }
                    }
                }
            }
            .pointerInput(track.keyframes, startFrame, endFrame) {
                detectDragGestures(
                    onDragStart = { offset ->
                        val span = (endFrame - startFrame).coerceAtLeast(1).toFloat()
                        val w = sizePx.width.toFloat().coerceAtLeast(1f)
                        val nearest = track.keyframes.minByOrNull { kf ->
                            val kx = ((kf.frame - startFrame) / span) * w
                            abs(kx - offset.x)
                        }
                        if (nearest != null) {
                            val kx = ((nearest.frame - startFrame) / span) * w
                            if (abs(kx - offset.x) <= 32f) {
                                draggingKey = nearest
                                dragFrame = nearest.frame
                                onSelectKeyframe(nearest)
                            }
                        }
                    },
                    onDrag = { change, _ ->
                        val dk = draggingKey ?: return@detectDragGestures
                        change.consume()
                        val span = (endFrame - startFrame).coerceAtLeast(1).toFloat()
                        val w = sizePx.width.toFloat().coerceAtLeast(1f)
                        val f = (startFrame + (change.position.x / w) * span).roundToInt()
                            .coerceIn(startFrame, endFrame)
                        dragFrame = f
                    },
                    onDragEnd = {
                        val dk = draggingKey
                        if (dk != null && dragFrame != dk.frame) {
                            onMoveKeyframe(dk, dragFrame)
                        }
                        draggingKey = null
                    },
                    onDragCancel = { draggingKey = null },
                )
            },
    ) {
        val span = (endFrame - startFrame).coerceAtLeast(1).toFloat()
        val w = size.width
        val cy = size.height * 0.5f

        val playX = ((currentFrame - startFrame) / span) * w
        drawLine(
            color = Color(0xFF38BDF8).copy(alpha = 0.55f),
            start = Offset(playX, 0f),
            end = Offset(playX, size.height),
            strokeWidth = 2f,
        )

        track.keyframes.forEach { kf ->
            val f = if (draggingKey?.id == kf.id) dragFrame else kf.frame
            val x = ((f - startFrame) / span) * w
            val selected = kf.id == selectedKeyId
            val r = if (selected) 8.5f else 6.5f
            val diamond = Path().apply {
                moveTo(x, cy - r)
                lineTo(x + r, cy)
                lineTo(x, cy + r)
                lineTo(x - r, cy)
                close()
            }
            drawPath(
                diamond,
                color = if (selected) Color(0xFFF59E0B) else Color(0xFF60A5FA),
            )
            drawPath(
                diamond,
                color = Color.White.copy(alpha = if (selected) 0.9f else 0.45f),
                style = Stroke(width = 1.3f),
            )
        }
    }
}

@Composable
private fun CurveGraphEditor(
    controller: ViewportController,
    zoomScale: Float,
    activeTrack: AnimTrackUi?,
    onEditKeyframe: (AnimTrackUi, KeyframeUi) -> Unit,
) {
    val tl = controller.timeline
    val tracks = controller.animTracks

    Column(modifier = Modifier.fillMaxSize()) {
        // Track switcher row
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(NovaColors.CardBg)
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 8.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(5.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (tracks.isEmpty()) {
                Text(
                    text = "No animated curves yet — tap + Key ◆ to create one",
                    color = NovaColors.TextSecondary,
                    fontSize = 11.sp,
                )
            } else {
                tracks.forEach { track ->
                    AnimChip(
                        label = "${controller.trackTargetName(track)} · ${track.channelLabel()}",
                        active = track.id == activeTrack?.id,
                        onClick = { controller.selectedTrackId = track.id },
                    )
                }
            }
        }

        if (activeTrack == null) {
            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                AnimChip(
                    label = "+ Key Selected Node @ F${tl.currentFrame.roundToInt()}",
                    active = true,
                    accent = Color(0xFFD97706),
                    onClick = { controller.keySelectedNodeTrs(7) },
                )
            }
            return
        }

        val kfs = activeTrack.keyframes
        val minValRaw = kfs.minOfOrNull { it.value } ?: 0f
        val maxValRaw = kfs.maxOfOrNull { it.value } ?: 1f
        val pad = max((maxValRaw - minValRaw) * 0.25f, 1.0f)
        val vMin = minValRaw - pad
        val vMax = maxValRaw + pad

        var canvasSize by remember { mutableStateOf(IntSize.Zero) }

        // Auto-select first keyframe of active track in Graph Editor so Bezier handles appear immediately
        val effectiveSelKeyId = controller.selectedKeyframeId
            ?.takeIf { sid -> activeTrack.keyframes.any { it.id == sid } }
            ?: activeTrack.keyframes.firstOrNull()?.id

        // Auto-frame the active track's keyframe span (adjusted by zoomScale) so curves fill the view clearly
        val minF = tl.startFrame.toFloat()
        val maxF = tl.endFrame.toFloat().coerceAtLeast(minF + 1f)
        val firstKf = kfs.minOfOrNull { it.frame.toFloat() } ?: minF
        val lastKf = kfs.maxOfOrNull { it.frame.toFloat() } ?: maxF
        val keySpan = (lastKf - firstKf).coerceAtLeast(12f)
        val baseSpan = min((maxF - minF), keySpan * 1.45f).coerceAtLeast(14f)
        val visibleSpan = (baseSpan / zoomScale).coerceAtLeast(6f).coerceAtMost(maxF - minF)
        val centerF = if (abs(tl.currentFrame - firstKf) <= baseSpan) {
            0.5f * (firstKf + lastKf).coerceAtLeast(firstKf + keySpan * 0.4f)
        } else {
            tl.currentFrame
        }
        val viewStartF = (centerF - visibleSpan * 0.45f)
            .coerceIn(minF, (maxF - visibleSpan).coerceAtLeast(minF))
        val viewEndF = viewStartF + visibleSpan

        // Drag state for responsive 60fps interaction before committing on dragEnd
        var dragMode by remember(activeTrack.id) { mutableIntStateOf(0) } // 1=key, 2=outHandle, 3=inHandle
        var dragKey by remember(activeTrack.id) { mutableStateOf<KeyframeUi?>(null) }
        var dragPreviewFrame by remember(activeTrack.id) { mutableIntStateOf(1) }
        var dragPreviewVal by remember(activeTrack.id) { mutableFloatStateOf(0f) }
        var dragPreviewHandle by remember(activeTrack.id) { mutableStateOf(Offset.Zero) }

        fun frameToX(f: Float, w: Float): Float = ((f - viewStartF) / (viewEndF - viewStartF)) * w
        fun valueToY(v: Float, h: Float): Float = h - ((v - vMin) / (vMax - vMin)) * h
        fun xToFrame(x: Float, w: Float): Float = viewStartF + (x / w.coerceAtLeast(1f)) * (viewEndF - viewStartF)
        fun yToValue(y: Float, h: Float): Float = vMin + ((h - y) / h.coerceAtLeast(1f)) * (vMax - vMin)

        // Effective track copy reflecting live drag preview
        val displayTrack = remember(activeTrack, dragMode, dragKey, dragPreviewFrame, dragPreviewVal, dragPreviewHandle) {
            val dk = dragKey
            if (dragMode == 0 || dk == null) {
                activeTrack
            } else {
                val updated = activeTrack.keyframes.map { k ->
                    if (k.id != dk.id) k else when (dragMode) {
                        1 -> k.copy(frame = dragPreviewFrame, value = dragPreviewVal)
                        2 -> k.copy(
                            interpolation = InterpolationUiMode.Bezier,
                            outHandleX = dragPreviewHandle.x,
                            outHandleY = dragPreviewHandle.y,
                        )
                        else -> k.copy(
                            interpolation = InterpolationUiMode.Bezier,
                            inHandleX = dragPreviewHandle.x,
                            inHandleY = dragPreviewHandle.y,
                        )
                    }
                }.sortedBy { it.frame }
                activeTrack.copy(keyframes = updated)
            }
        }

        Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
            Canvas(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color(0xFF101319))
                    .onSizeChanged { canvasSize = it }
                    .pointerInput(activeTrack, viewStartF, viewEndF, vMin, vMax) {
                        detectTapGestures { offset ->
                            val w = canvasSize.width.toFloat().coerceAtLeast(1f)
                            val h = canvasSize.height.toFloat().coerceAtLeast(1f)
                            val hitKey = activeTrack.keyframes.minByOrNull { k ->
                                val dx = frameToX(k.frame.toFloat(), w) - offset.x
                                val dy = valueToY(k.value, h) - offset.y
                                dx * dx + dy * dy
                            }
                            if (hitKey != null) {
                                val dx = frameToX(hitKey.frame.toFloat(), w) - offset.x
                                val dy = valueToY(hitKey.value, h) - offset.y
                                if (dx * dx + dy * dy <= 42f * 42f) {
                                    controller.selectedKeyframeId = hitKey.id
                                    controller.setCurrentFrame(hitKey.frame.toFloat())
                                    return@detectTapGestures
                                }
                            }
                            controller.setCurrentFrame(xToFrame(offset.x, w).coerceIn(minF, maxF))
                        }
                    }
                    .pointerInput(activeTrack, controller.selectedKeyframeId, viewStartF, viewEndF, vMin, vMax) {
                        detectDragGestures(
                            onDragStart = { offset ->
                                val w = canvasSize.width.toFloat().coerceAtLeast(1f)
                                val h = canvasSize.height.toFloat().coerceAtLeast(1f)
                                val sorted = activeTrack.keyframes.sortedBy { it.frame }
                                val selIdx = sorted.indexOfFirst { it.id == effectiveSelKeyId }
                                if (selIdx >= 0) {
                                    val selK = sorted[selIdx]
                                    if (selIdx + 1 < sorted.size) {
                                        val nextK = sorted[selIdx + 1]
                                        val dt = (nextK.frame - selK.frame).toFloat().coerceAtLeast(1f)
                                        val hx = frameToX(selK.frame + selK.outHandleX * dt, w)
                                        val hy = valueToY(selK.value + selK.outHandleY, h)
                                        if ((hx - offset.x) * (hx - offset.x) + (hy - offset.y) * (hy - offset.y) <= 44f * 44f) {
                                            dragMode = 2
                                            dragKey = selK
                                            dragPreviewHandle = Offset(selK.outHandleX, selK.outHandleY)
                                            return@detectDragGestures
                                        }
                                    }
                                    if (selIdx - 1 >= 0) {
                                        val prevK = sorted[selIdx - 1]
                                        val dt = (selK.frame - prevK.frame).toFloat().coerceAtLeast(1f)
                                        val hx = frameToX(selK.frame + selK.inHandleX * dt, w)
                                        val hy = valueToY(selK.value + selK.inHandleY, h)
                                        if ((hx - offset.x) * (hx - offset.x) + (hy - offset.y) * (hy - offset.y) <= 44f * 44f) {
                                            dragMode = 3
                                            dragKey = selK
                                            dragPreviewHandle = Offset(selK.inHandleX, selK.inHandleY)
                                            return@detectDragGestures
                                        }
                                    }
                                }
                                val hitKey = sorted.minByOrNull { k ->
                                    val dx = frameToX(k.frame.toFloat(), w) - offset.x
                                    val dy = valueToY(k.value, h) - offset.y
                                    dx * dx + dy * dy
                                }
                                if (hitKey != null) {
                                    val dx = frameToX(hitKey.frame.toFloat(), w) - offset.x
                                    val dy = valueToY(hitKey.value, h) - offset.y
                                    if (dx * dx + dy * dy <= 44f * 44f) {
                                        controller.selectedKeyframeId = hitKey.id
                                        dragMode = 1
                                        dragKey = hitKey
                                        dragPreviewFrame = hitKey.frame
                                        dragPreviewVal = hitKey.value
                                    }
                                }
                            },
                            onDrag = { change, _ ->
                                val dk = dragKey ?: return@detectDragGestures
                                change.consume()
                                val w = canvasSize.width.toFloat().coerceAtLeast(1f)
                                val h = canvasSize.height.toFloat().coerceAtLeast(1f)
                                val fDrag = xToFrame(change.position.x, w)
                                val vDrag = yToValue(change.position.y, h)
                                val sorted = activeTrack.keyframes.sortedBy { it.frame }
                                val idx = sorted.indexOfFirst { it.id == dk.id }
                                when (dragMode) {
                                    1 -> {
                                        dragPreviewFrame = fDrag.roundToInt().coerceIn(tl.startFrame, tl.endFrame)
                                        dragPreviewVal = vDrag
                                    }
                                    2 -> if (idx >= 0 && idx + 1 < sorted.size) {
                                        val dt = (sorted[idx + 1].frame - dk.frame).toFloat().coerceAtLeast(1f)
                                        dragPreviewHandle = Offset(
                                            ((fDrag - dk.frame) / dt).coerceIn(0.05f, 0.95f),
                                            vDrag - dk.value,
                                        )
                                    }
                                    3 -> if (idx > 0) {
                                        val dt = (dk.frame - sorted[idx - 1].frame).toFloat().coerceAtLeast(1f)
                                        dragPreviewHandle = Offset(
                                            ((fDrag - dk.frame) / dt).coerceIn(-0.95f, -0.05f),
                                            vDrag - dk.value,
                                        )
                                    }
                                }
                            },
                            onDragEnd = {
                                val dk = dragKey
                                if (dk != null) {
                                    when (dragMode) {
                                        1 -> controller.updateKeyframe(
                                            activeTrack.id, dk.id, dragPreviewFrame, dragPreviewVal,
                                            dk.interpolation, dk.inHandleX, dk.inHandleY, dk.outHandleX, dk.outHandleY,
                                        )
                                        2 -> controller.updateKeyframe(
                                            activeTrack.id, dk.id, dk.frame, dk.value,
                                            InterpolationUiMode.Bezier,
                                            dk.inHandleX, dk.inHandleY, dragPreviewHandle.x, dragPreviewHandle.y,
                                        )
                                        3 -> controller.updateKeyframe(
                                            activeTrack.id, dk.id, dk.frame, dk.value,
                                            InterpolationUiMode.Bezier,
                                            dragPreviewHandle.x, dragPreviewHandle.y, dk.outHandleX, dk.outHandleY,
                                        )
                                    }
                                }
                                dragMode = 0
                                dragKey = null
                            },
                            onDragCancel = {
                                dragMode = 0
                                dragKey = null
                            },
                        )
                    },
            ) {
                val w = size.width
                val h = size.height

                // Grid lines
                for (g in 1..3) {
                    val gy = h * g / 4f
                    drawLine(
                        color = Color(0xFF1E2430),
                        start = Offset(0f, gy),
                        end = Offset(w, gy),
                        strokeWidth = 1f,
                    )
                }
                val zeroY = valueToY(0f, h)
                if (zeroY in 0f..h) {
                    drawLine(
                        color = Color(0xFF334155),
                        start = Offset(0f, zeroY),
                        end = Offset(w, zeroY),
                        strokeWidth = 1.5f,
                    )
                }

                // Playhead line
                val playX = frameToX(tl.currentFrame, w)
                drawLine(
                    color = Color(0xFF38BDF8),
                    start = Offset(playX, 0f),
                    end = Offset(playX, h),
                    strokeWidth = 2f,
                )

                // Exact curve evaluation matching Rust's AnimTrack evaluator
                val samples = 140
                val curvePath = Path()
                for (s in 0..samples) {
                    val f = viewStartF + (s.toFloat() / samples.toFloat()) * (viewEndF - viewStartF)
                    val v = evalTrackUiAt(displayTrack.keyframes, f)
                    val px = frameToX(f, w)
                    val py = valueToY(v, h)
                    if (s == 0) curvePath.moveTo(px, py) else curvePath.lineTo(px, py)
                }
                drawPath(
                    path = curvePath,
                    color = Color(0xFF60A5FA),
                    style = Stroke(width = 3.2f, cap = StrokeCap.Round),
                )

                // Draw Bezier handles for selected keyframe and all keyframe points
                val sortedKfs = displayTrack.keyframes.sortedBy { it.frame }
                sortedKfs.forEachIndexed { idx, k ->
                    val kx = frameToX(k.frame.toFloat(), w)
                    val ky = valueToY(k.value, h)
                    val isSel = k.id == effectiveSelKeyId

                    if (isSel) {
                        if (idx + 1 < sortedKfs.size) {
                            val dt = (sortedKfs[idx + 1].frame - k.frame).toFloat().coerceAtLeast(1f)
                            val hx = frameToX(k.frame + k.outHandleX * dt, w)
                            val hy = valueToY(k.value + k.outHandleY, h)
                            drawLine(Color(0xFFF59E0B), Offset(kx, ky), Offset(hx, hy), strokeWidth = 2f)
                            drawCircle(Color(0xFFF59E0B), radius = 7f, center = Offset(hx, hy))
                            drawCircle(Color.White, radius = 3f, center = Offset(hx, hy))
                        }
                        if (idx - 1 >= 0) {
                            val dt = (k.frame - sortedKfs[idx - 1].frame).toFloat().coerceAtLeast(1f)
                            val hx = frameToX(k.frame + k.inHandleX * dt, w)
                            val hy = valueToY(k.value + k.inHandleY, h)
                            drawLine(Color(0xFFF59E0B), Offset(kx, ky), Offset(hx, hy), strokeWidth = 2f)
                            drawCircle(Color(0xFFF59E0B), radius = 7f, center = Offset(hx, hy))
                            drawCircle(Color.White, radius = 3f, center = Offset(hx, hy))
                        }
                    }

                    drawCircle(
                        color = if (isSel) Color(0xFFF59E0B) else Color(0xFF93C5FD),
                        radius = if (isSel) 9f else 6.5f,
                        center = Offset(kx, ky),
                    )
                    drawCircle(
                        color = Color(0xFF0B0E14),
                        radius = if (isSel) 3.5f else 2.5f,
                        center = Offset(kx, ky),
                    )
                }
            }

            // Min / Max value overlay labels
            Text(
                text = String.format(Locale.US, "%.2f", vMax),
                color = NovaColors.TextMuted,
                fontSize = 9.sp,
                fontFamily = FontFamily.Monospace,
                modifier = Modifier.align(Alignment.TopStart).padding(4.dp),
            )
            Text(
                text = String.format(Locale.US, "%.2f", vMin),
                color = NovaColors.TextMuted,
                fontSize = 9.sp,
                fontFamily = FontFamily.Monospace,
                modifier = Modifier.align(Alignment.BottomStart).padding(4.dp),
            )
        }

        val selKey = activeTrack.keyframes.firstOrNull { it.id == effectiveSelKeyId }
        if (selKey != null) {
            HorizontalDivider(color = NovaColors.PanelBorder)
            SelectedKeyframeBar(
                trackLabel = activeTrack.channelLabel(),
                keyframe = selKey,
                onEdit = { onEditKeyframe(activeTrack, selKey) },
                onDelete = { controller.deleteKeyframe(activeTrack.id, selKey.id) },
            )
        }
    }
}

private fun evalTrackUiAt(keyframes: List<KeyframeUi>, frame: Float): Float {
    if (keyframes.isEmpty()) return 0f
    val sorted = keyframes.sortedBy { it.frame }
    if (frame <= sorted.first().frame) return sorted.first().value
    if (frame >= sorted.last().frame) return sorted.last().value
    val seg = sorted.indexOfLast { it.frame <= frame }.coerceIn(0, sorted.lastIndex - 1)
    val a = sorted[seg]
    val b = sorted[seg + 1]
    val dt = (b.frame - a.frame).toFloat().coerceAtLeast(1e-4f)
    val u = ((frame - a.frame) / dt).coerceIn(0f, 1f)
    return when (a.interpolation) {
        InterpolationUiMode.Constant -> a.value
        InterpolationUiMode.Linear -> a.value + (b.value - a.value) * u
        InterpolationUiMode.EaseIn -> a.value + (b.value - a.value) * (u * u * u)
        InterpolationUiMode.EaseOut -> {
            val inv = 1f - u
            a.value + (b.value - a.value) * (1f - inv * inv * inv)
        }
        InterpolationUiMode.EaseInOut -> {
            val s = u * u * (3f - 2f * u)
            a.value + (b.value - a.value) * s
        }
        InterpolationUiMode.Bezier -> {
            val p0x = a.frame.toFloat()
            val p0y = a.value
            val p3x = b.frame.toFloat()
            val p3y = b.value
            val p1x = p0x + a.outHandleX.coerceIn(0.01f, 0.99f) * dt
            val p1y = p0y + a.outHandleY
            val p2x = p3x + b.inHandleX.coerceIn(-0.99f, -0.01f) * dt
            val p2y = p3y + b.inHandleY
            var lo = 0f
            var hi = 1f
            for (i in 0 until 14) {
                val mid = 0.5f * (lo + hi)
                val inv = 1f - mid
                val bx = inv.pow(3) * p0x + 3f * inv * inv * mid * p1x + 3f * inv * mid * mid * p2x + mid.pow(3) * p3x
                if (bx < frame) lo = mid else hi = mid
            }
            val t = 0.5f * (lo + hi)
            val inv = 1f - t
            inv.pow(3) * p0y + 3f * inv * inv * t * p1y + 3f * inv * t * t * p2y + t.pow(3) * p3y
        }
    }
}

@Composable
private fun SelectedKeyframeBar(
    trackLabel: String,
    keyframe: KeyframeUi,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(NovaColors.CardBg)
            .horizontalScroll(rememberScrollState())
            .padding(horizontal = 8.dp, vertical = 5.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = String.format(
                Locale.US,
                "%s @ F%d = %.3f (%s)",
                trackLabel,
                keyframe.frame,
                keyframe.value,
                keyframe.interpolation.label,
            ),
            color = NovaColors.TextPrimary,
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
        )
        AnimChip(label = "Edit Key", active = false, onClick = onEdit)
        AnimChip(label = "Delete Key", active = false, accent = NovaColors.ErrorRed, onClick = onDelete)
    }
}

@Composable
private fun TimelineSettingsDialog(
    startFrame: Int,
    endFrame: Int,
    fps: Float,
    onDismiss: () -> Unit,
    onConfirm: (Int, Int, Float) -> Unit,
) {
    var startTxt by remember { mutableStateOf(startFrame.toString()) }
    var endTxt by remember { mutableStateOf(endFrame.toString()) }
    var fpsVal by remember { mutableFloatStateOf(fps) }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NovaColors.PanelBg,
        titleContentColor = NovaColors.TextPrimary,
        textContentColor = NovaColors.TextSecondary,
        title = { Text("Timeline Settings") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = startTxt,
                    onValueChange = { startTxt = it },
                    label = { Text("Start Frame") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                )
                OutlinedTextField(
                    value = endTxt,
                    onValueChange = { endTxt = it },
                    label = { Text("End Frame") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf(24f, 30f, 60f).forEach { candidate ->
                        AnimChip(
                            label = "${candidate.toInt()} fps",
                            active = abs(fpsVal - candidate) < 0.5f,
                            onClick = { fpsVal = candidate },
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = {
                val s = startTxt.toIntOrNull() ?: startFrame
                val e = endTxt.toIntOrNull() ?: endFrame
                onConfirm(s, max(e, s + 1), fpsVal)
            }) { Text("Apply") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}

@Composable
private fun EditKeyframeDialog(
    trackLabel: String,
    keyframe: KeyframeUi,
    onDismiss: () -> Unit,
    onDelete: () -> Unit,
    onConfirm: (Int, Float, InterpolationUiMode) -> Unit,
) {
    var frameTxt by remember { mutableStateOf(keyframe.frame.toString()) }
    var valueTxt by remember { mutableStateOf(String.format(Locale.US, "%.4f", keyframe.value)) }
    var interp by remember { mutableStateOf(keyframe.interpolation) }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NovaColors.PanelBg,
        titleContentColor = NovaColors.TextPrimary,
        textContentColor = NovaColors.TextSecondary,
        title = { Text("Keyframe: $trackLabel") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = frameTxt,
                    onValueChange = { frameTxt = it },
                    label = { Text("Frame") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                )
                OutlinedTextField(
                    value = valueTxt,
                    onValueChange = { valueTxt = it },
                    label = { Text("Value") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    singleLine = true,
                )
                Text("Interpolation", color = NovaColors.TextSecondary, fontSize = 11.sp)
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                ) {
                    InterpolationUiMode.entries.forEach { m ->
                        AnimChip(
                            label = m.label,
                            active = interp == m,
                            onClick = { interp = m },
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = {
                val f = frameTxt.toIntOrNull() ?: keyframe.frame
                val v = valueTxt.toFloatOrNull() ?: keyframe.value
                onConfirm(f, v, interp)
            }) { Text("Save") }
        },
        dismissButton = {
            Row {
                TextButton(onClick = onDelete) { Text("Delete", color = NovaColors.ErrorRed) }
                TextButton(onClick = onDismiss) { Text("Cancel") }
            }
        },
    )
}

@Composable
private fun AnimChip(
    label: String,
    active: Boolean,
    enabled: Boolean = true,
    accent: Color = NovaColors.AccentBlue,
    onClick: () -> Unit,
) {
    Surface(
        color = if (active) accent.copy(alpha = 0.24f) else NovaColors.CardBg,
        shape = RoundedCornerShape(6.dp),
        border = BorderStroke(1.dp, if (active) accent else NovaColors.PanelBorder),
        modifier = Modifier
            .clip(RoundedCornerShape(6.dp))
            .clickable(enabled = enabled, onClick = onClick),
    ) {
        Text(
            text = label,
            color = when {
                !enabled -> NovaColors.TextMuted.copy(alpha = 0.45f)
                active -> Color.White
                else -> NovaColors.TextPrimary
            },
            fontSize = 11.sp,
            fontWeight = FontWeight.Medium,
            maxLines = 1,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 5.dp),
        )
    }
}
